<script lang="ts">
  import { goto } from "$app/navigation";
  import type { ImportBatchResult, ImportLinkRow } from "$lib/api/links";
  import { linksApi } from "$lib/api/links";
  import type { UsageResponse } from "$lib/types/api";
  import { SvelteMap } from "svelte/reactivity";

  const { data }: { data: { user: unknown; usage: UsageResponse | null } } =
    $props();

  // ── Step state ────────────────────────────────────────────────────────────
  let step = $state(1); // 1-5
  const STEPS = ["Upload", "Map Columns", "Preview", "Settings", "Importing"];

  // ── Step 1: Upload ────────────────────────────────────────────────────────
  let isDragging = $state(false);
  let parseError = $state("");
  let rawRows = $state<Record<string, string>[]>([]); // parsed CSV rows (header → value map)
  let csvHeaders = $state<string[]>([]);

  // ── Step 2: Column mapping ────────────────────────────────────────────────
  type ColKey =
    | "destination_url"
    | "short_code"
    | "title"
    | "tags"
    | "expires_at";

  const COLUMN_DEFS: {
    key: ColKey;
    label: string;
    required: boolean;
    aliases: string[];
  }[] = [
    {
      key: "destination_url",
      label: "Destination URL",
      required: true,
      aliases: [
        "destination_url",
        "url",
        "long_url",
        "destination",
        "target_url",
        "link",
        "href"
      ]
    },
    {
      key: "short_code",
      label: "Short Code",
      required: false,
      aliases: [
        "short_code",
        "short_url",
        "slug",
        "alias",
        "custom_code",
        "code",
        "keyword",
        "link",
        "url",
        "full_url",
        "short_link"
      ]
    },
    {
      key: "title",
      label: "Title",
      required: false,
      aliases: ["title", "name", "description", "label", "link_name"]
    },
    {
      key: "tags",
      label: "Tags (comma or pipe-separated)",
      required: false,
      aliases: ["tags", "tag", "labels", "categories"]
    },
    {
      key: "expires_at",
      label: "Expires At (ISO date)",
      required: false,
      aliases: [
        "expires_at",
        "expiry",
        "expiration",
        "expire_date",
        "valid_until"
      ]
    }
  ];

  let columnMap = $state<Record<ColKey, string>>({
    destination_url: "",
    short_code: "",
    title: "",
    tags: "",
    expires_at: ""
  });

  // ── Step 3: Preview ───────────────────────────────────────────────────────
  const PREVIEW_COUNT = 10;
  let validationErrors = $state<Map<number, string>>(new Map());

  // ── Step 4: Settings ──────────────────────────────────────────────────────
  let skipInvalidRows = $state(true);

  // ── Step 5: Progress + Results ────────────────────────────────────────────
  const BATCH_SIZE = 5;
  const MAX_SHORT_CODE_LENGTH = 100;
  let progress = $state(0); // 0-100
  let importDone = $state(false);
  let totalCreated = $state(0);
  let totalSkipped = $state(0);
  let totalFailed = $state(0);
  let allErrors = $state<
    { row: number; destination_url: string; reason: string }[]
  >([]);
  let allWarnings = $state<
    { row: number; destination_url: string; reason: string }[]
  >([]);

  // ── Derived ───────────────────────────────────────────────────────────────
  const isProOrAbove = $derived(
    data.usage?.tier === "pro" ||
      data.usage?.tier === "business" ||
      data.usage?.tier === "unlimited"
  );

  const mappingResult = $derived.by(() => {
    return rawRows.map((row) => {
      const destCol = columnMap.destination_url;
      const scCol = columnMap.short_code;
      const titleCol = columnMap.title;
      const tagsCol = columnMap.tags;
      const expiresCol = columnMap.expires_at;

      const destination_url = destCol ? (row[destCol] ?? "").trim() : "";
      let rawCode =
        scCol && isProOrAbove ? extractShortCode(row[scCol] ?? "") : "";
      let wasTruncated = false;
      if (rawCode.length > MAX_SHORT_CODE_LENGTH) {
        rawCode = rawCode.slice(0, MAX_SHORT_CODE_LENGTH);
        wasTruncated = true;
      }
      const short_code = rawCode || undefined;
      const title = titleCol
        ? (row[titleCol] ?? "").trim() || undefined
        : undefined;
      const tagsRaw = tagsCol ? (row[tagsCol] ?? "").trim() : "";
      const tags = parseTags(tagsRaw);
      const expiresRaw = expiresCol ? (row[expiresCol] ?? "").trim() : "";
      const expires_at = expiresRaw ? isoToTimestamp(expiresRaw) : undefined;

      return {
        destination_url,
        short_code,
        title,
        tags,
        expires_at,
        wasTruncated
      };
    });
  });

  const mappedRows = $derived(
    mappingResult.map(
      ({ destination_url, short_code, title, tags, expires_at }) => ({
        destination_url,
        short_code,
        title,
        tags,
        expires_at
      })
    ) as ImportLinkRow[]
  );

  const truncatedCount = $derived(
    mappingResult.filter((r) => r.wasTruncated).length
  );

  const validRows = $derived.by((): ImportLinkRow[] => {
    if (skipInvalidRows) {
      return mappedRows.filter((r) => isValidUrl(r.destination_url));
    }
    return mappedRows;
  });

  const remainingQuota = $derived.by((): number | null => {
    if (!data.usage?.limits.max_links_per_month) return null;
    return (
      data.usage.limits.max_links_per_month -
      data.usage.usage.links_created_this_month
    );
  });

  // ── CSV parser ────────────────────────────────────────────────────────────
  function parseCSV(text: string): {
    headers: string[];
    rows: Record<string, string>[];
  } {
    const lines = text.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
    const records = splitCSVIntoRecords(lines);
    if (records.length === 0) return { headers: [], rows: [] };

    const headers = records[0].map((h) => h.trim());
    const rows: Record<string, string>[] = [];

    for (let i = 1; i < records.length; i++) {
      const values = records[i];
      if (values.every((v) => v.trim() === "")) continue; // skip blank lines
      const row: Record<string, string> = {};
      headers.forEach((h, idx) => {
        row[h] = values[idx] ?? "";
      });
      rows.push(row);
    }
    return { headers, rows };
  }

  function splitCSVIntoRecords(text: string): string[][] {
    const records: string[][] = [];
    let currentRecord: string[] = [];
    let currentField = "";
    let inQuotes = false;
    let i = 0;

    while (i < text.length) {
      const ch = text[i];

      if (inQuotes) {
        if (ch === '"') {
          if (text[i + 1] === '"') {
            currentField += '"';
            i += 2;
            continue;
          } else {
            inQuotes = false;
          }
        } else {
          currentField += ch;
        }
      } else {
        if (ch === '"') {
          inQuotes = true;
        } else if (ch === ",") {
          currentRecord.push(currentField);
          currentField = "";
        } else if (ch === "\n") {
          currentRecord.push(currentField);
          currentField = "";
          records.push(currentRecord);
          currentRecord = [];
          i++;
          continue;
        } else {
          currentField += ch;
        }
      }
      i++;
    }

    // Push last field/record
    if (currentField !== "" || currentRecord.length > 0) {
      currentRecord.push(currentField);
      records.push(currentRecord);
    }

    return records;
  }

  // ── Helpers ───────────────────────────────────────────────────────────────
  function parseTags(tagsRaw: string): string[] | undefined {
    if (!tagsRaw.trim()) return undefined;

    // Remove surrounding quotes if present (Dub format)
    const cleaned = tagsRaw.replace(/^"(.*)"$/, "$1").trim();

    // Try comma separator first (Dub format), then pipe separator (our format)
    const separator = cleaned.includes(",") ? "," : "|";

    const tags = cleaned
      .split(separator)
      .map((t) => t.trim())
      .filter((t) => t.length > 0);

    return tags.length > 0 ? tags : undefined;
  }

  function extractShortCode(value: string): string {
    const trimmed = value.trim();
    // If it looks like a full URL, extract the last path segment
    if (trimmed.includes("/")) {
      const parts = trimmed.split("/");
      // Remove empty parts caused by trailing/leading slashes
      const filtered = parts.filter(Boolean);
      return filtered[filtered.length - 1] || "";
    }
    return trimmed;
  }

  function isValidUrl(url: string): boolean {
    try {
      const u = new URL(url);
      return u.protocol === "http:" || u.protocol === "https:";
    } catch {
      return false;
    }
  }

  function isoToTimestamp(iso: string): number | undefined {
    const d = new Date(iso);
    return isNaN(d.getTime()) ? undefined : Math.floor(d.getTime() / 1000);
  }

  function autoDetectColumns(headers: string[]) {
    const lc = headers.map((h) => h.toLowerCase().trim());
    const newMap = { ...columnMap };

    for (const def of COLUMN_DEFS) {
      for (const alias of def.aliases) {
        const idx = lc.indexOf(alias);
        if (idx !== -1) {
          newMap[def.key] = headers[idx];
          break;
        }
      }
    }
    columnMap = newMap;
  }

  function validatePreviewRows() {
    const errors = new SvelteMap<number, string>();
    const preview = mappedRows.slice(0, PREVIEW_COUNT);
    preview.forEach((row, i) => {
      if (!row.destination_url) {
        errors.set(i, "Missing destination URL");
      } else if (!isValidUrl(row.destination_url)) {
        errors.set(i, `Invalid URL: "${row.destination_url}"`);
      }
    });
    validationErrors = errors;
  }

  // ── File handling ─────────────────────────────────────────────────────────
  function processFile(file: File) {
    parseError = "";
    if (!file.name.endsWith(".csv") && file.type !== "text/csv") {
      parseError = "Please upload a CSV file.";
      return;
    }
    const reader = new FileReader();
    reader.onload = (e) => {
      const text = e.target?.result as string;
      try {
        const { headers, rows } = parseCSV(text);
        if (headers.length === 0) {
          parseError = "CSV file appears to be empty or malformed.";
          return;
        }
        csvHeaders = headers;
        rawRows = rows;
        autoDetectColumns(headers);
        step = 2;
      } catch {
        parseError = "Failed to parse CSV file. Please check the format.";
      }
    };
    reader.readAsText(file);
  }

  function handleFileInput(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (file) processFile(file);
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
    const file = e.dataTransfer?.files?.[0];
    if (file) processFile(file);
  }

  // ── Step navigation ───────────────────────────────────────────────────────
  function goToPreview() {
    if (!columnMap.destination_url) {
      parseError = 'You must map a column to "Destination URL".';
      return;
    }
    parseError = "";
    validatePreviewRows();
    step = 3;
  }

  function goToSettings() {
    step = 4;
  }

  async function startImport() {
    step = 5;
    progress = 0;
    totalCreated = 0;
    totalSkipped = 0;
    totalFailed = 0;
    allErrors = [];
    allWarnings = [];
    importDone = false;

    const rows = validRows;
    if (rows.length === 0) {
      importDone = true;
      return;
    }

    // Yield to the browser so Svelte can paint the 0% state before the first request
    await new Promise((r) => setTimeout(r, 0));

    const chunks: ImportLinkRow[][] = [];
    for (let i = 0; i < rows.length; i += BATCH_SIZE) {
      chunks.push(rows.slice(i, i + BATCH_SIZE));
    }

    for (let ci = 0; ci < chunks.length; ci++) {
      try {
        const result: ImportBatchResult = await linksApi.importBatch(
          chunks[ci]
        );
        totalCreated += result.created;
        totalSkipped += result.skipped;
        totalFailed += result.failed;
        // Offset row numbers by chunk position
        const offset = ci * BATCH_SIZE;
        for (const err of result.errors) {
          allErrors = [...allErrors, { ...err, row: err.row + offset }];
        }
        for (const warn of result.warnings ?? []) {
          allWarnings = [...allWarnings, { ...warn, row: warn.row + offset }];
        }
      } catch {
        totalFailed += chunks[ci].length;
      }
      progress = Math.round(((ci + 1) / chunks.length) * 100);
    }

    importDone = true;
  }

  function downloadErrorReport() {
    const header = "row,destination_url,reason\n";
    const lines = allErrors
      .map(
        (e) =>
          `${e.row},${e.destination_url.includes(",") ? `"${e.destination_url}"` : e.destination_url},"${e.reason.replace(/"/g, '""')}"`
      )
      .join("\n");
    const blob = new Blob([header + lines], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "import-errors.csv";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }
</script>

<svelte:head>
  <title>Import Links - Rushomon</title>
  <style>
    @keyframes progress-stripes {
      from {
        background-position: 40px 0;
      }
      to {
        background-position: 0 0;
      }
    }
    .progress-animated {
      background-image: linear-gradient(
        45deg,
        rgba(255, 255, 255, 0.15) 25%,
        transparent 25%,
        transparent 50%,
        rgba(255, 255, 255, 0.15) 50%,
        rgba(255, 255, 255, 0.15) 75%,
        transparent 75%,
        transparent
      );
      background-size: 40px 40px;
      animation: progress-stripes 1s linear infinite;
    }
  </style>
</svelte:head>

<div class="min-h-screen bg-gray-50">
  <!-- Page header with merged step indicator -->
  <div class="border-b border-gray-200 bg-white">
    <div class="max-w-6xl mx-auto px-6 py-4">
      <!-- Desktop: title left, steps right -->
      <div
        class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4"
      >
        <div>
          <h1 class="text-xl font-semibold text-gray-900">Import Links</h1>
          <p class="text-sm text-gray-500 mt-0.5">
            Bulk-import short links from a CSV file
          </p>
        </div>
        <div class="flex items-center justify-center sm:justify-end">
          <ol class="flex items-center gap-0">
            {#each STEPS as label, i (i)}
              {@const n = i + 1}
              {@const done = step > n}
              {@const active = step === n}
              <li
                class="flex items-center {i < STEPS.length - 1 ? 'flex-1' : ''}"
              >
                <span
                  class="flex items-center gap-1.5 text-sm font-medium whitespace-nowrap
									{active ? 'text-orange-600' : done ? 'text-green-600' : 'text-gray-400'}"
                >
                  <span
                    class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold
										{active
                      ? 'bg-orange-100 text-orange-600 ring-2 ring-orange-500'
                      : done
                        ? 'bg-green-100 text-green-600'
                        : 'bg-gray-100 text-gray-400'}"
                  >
                    {#if done}✓{:else}{n}{/if}
                  </span>
                  <span class="hidden sm:inline">{label}</span>
                </span>
                {#if i < STEPS.length - 1}
                  <div class="flex-1 h-px bg-gray-200 mx-2"></div>
                {/if}
              </li>
            {/each}
          </ol>
        </div>
      </div>
    </div>
  </div>

  <main class="max-w-6xl mx-auto px-6 py-8">
    <!-- ── Step 1: Upload ── -->
    {#if step === 1}
      <div class="bg-white rounded-2xl border border-gray-200 p-8">
        <h2 class="text-lg font-semibold text-gray-900 mb-1">
          Upload CSV File
        </h2>
        <p class="text-sm text-gray-500 mb-6">
          Your CSV must have a header row. We'll auto-detect common column
          names.
        </p>

        <!-- Drag-and-drop zone -->
        <label
          class="block border-2 border-dashed rounded-xl p-10 text-center cursor-pointer transition-colors
					{isDragging
            ? 'border-orange-400 bg-orange-50'
            : 'border-gray-300 hover:border-orange-400 hover:bg-orange-50/50'}"
          ondragover={(e) => {
            e.preventDefault();
            isDragging = true;
          }}
          ondragleave={() => (isDragging = false)}
          ondrop={handleDrop}
        >
          <input
            type="file"
            accept=".csv,text/csv"
            class="sr-only"
            onchange={handleFileInput}
          />
          <svg
            class="w-10 h-10 mx-auto mb-3 {isDragging
              ? 'text-orange-500'
              : 'text-gray-400'}"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="1.5"
              d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
            />
          </svg>
          <p class="text-sm font-medium text-gray-700">
            {isDragging
              ? "Drop it here!"
              : "Drag and drop a CSV file, or click to browse"}
          </p>
          <p class="text-xs text-gray-400 mt-1">
            Supports .csv files up to any size (batched in chunks of 5)
          </p>
        </label>

        {#if parseError}
          <p class="mt-4 text-sm text-red-600">{parseError}</p>
        {/if}

        <!-- Template download hint -->
        <div class="mt-6 p-4 bg-blue-50 rounded-xl text-sm text-blue-700">
          <strong>Tip:</strong> Your CSV must include at least a destination URL
          column. Optional columns: short code (Pro+; full URLs like
          https://dub.co/abc are auto‑extracted to "abc", supports letters,
          numbers, hyphens, and forward slashes), title, tags (comma or
          pipe-separated:
          <code class="font-mono bg-blue-100 px-1 rounded">tag1,tag2</code>
          or
          <code class="font-mono bg-blue-100 px-1 rounded">tag1|tag2</code>),
          expires_at (ISO 8601).
        </div>
      </div>
    {/if}

    <!-- ── Step 2: Column Mapping ── -->
    {#if step === 2}
      <div class="bg-white rounded-2xl border border-gray-200 p-8">
        <h2 class="text-lg font-semibold text-gray-900 mb-1">Map Columns</h2>
        <p class="text-sm text-gray-500 mb-6">
          {rawRows.length} rows detected. Map your CSV columns to the fields below.
        </p>

        <div class="space-y-4">
          {#each COLUMN_DEFS as def (def.key)}
            {@const isShortCode = def.key === "short_code"}
            {@const disabled = isShortCode && !isProOrAbove}
            <div class="flex items-center gap-4">
              <div class="w-48 shrink-0">
                <span class="text-sm font-medium text-gray-800"
                  >{def.label}</span
                >
                {#if def.required}
                  <span class="ml-1 text-red-500 text-xs">*</span>
                {/if}
                {#if disabled}
                  <span class="ml-1 text-xs text-gray-400">(Pro+)</span>
                {/if}
              </div>
              <select
                bind:value={columnMap[def.key]}
                {disabled}
                class="flex-1 text-sm border border-gray-200 rounded-lg px-3 py-2 bg-white focus:outline-none focus:ring-2 focus:ring-orange-400 disabled:opacity-40 disabled:cursor-not-allowed"
              >
                <option value="">— not mapped —</option>
                {#each csvHeaders as header (header)}
                  <option value={header}>{header}</option>
                {/each}
              </select>
            </div>
          {/each}
        </div>

        {#if parseError}
          <p class="mt-4 text-sm text-red-600">{parseError}</p>
        {/if}

        <div class="flex items-center justify-between mt-8">
          <button
            onclick={() => {
              step = 1;
              parseError = "";
            }}
            class="text-sm text-gray-500 hover:text-gray-700"
          >
            ← Back
          </button>
          <button
            onclick={goToPreview}
            class="bg-gradient-to-r from-orange-500 to-orange-600 text-white px-6 py-2 rounded-lg font-semibold text-sm hover:shadow-md transition-all"
          >
            Preview rows →
          </button>
        </div>
      </div>
    {/if}

    <!-- ── Step 3: Preview ── -->
    {#if step === 3}
      {@const preview = mappingResult.slice(0, PREVIEW_COUNT)}
      <div class="bg-white rounded-2xl border border-gray-200 p-8">
        <h2 class="text-lg font-semibold text-gray-900 mb-1">Preview</h2>
        <p class="text-sm text-gray-500 mb-6">
          Showing first {Math.min(PREVIEW_COUNT, rawRows.length)} of {rawRows.length}
          rows.
          {#if validationErrors.size > 0}{validationErrors.size} row(s) have issues.{:else}All
            previewed rows look valid.{/if}
        </p>

        <div class="overflow-x-auto rounded-xl border border-gray-200">
          <table class="w-full text-sm">
            <thead
              class="bg-gray-50 text-xs font-semibold text-gray-500 uppercase tracking-wide"
            >
              <tr>
                <th class="px-3 py-2 text-left">#</th>
                <th class="px-3 py-2 text-left">Destination URL</th>
                {#if columnMap.short_code && isProOrAbove}
                  <th class="px-3 py-2 text-left">Short Code</th>
                {/if}
                {#if columnMap.title}
                  <th class="px-3 py-2 text-left">Title</th>
                {/if}
                <th class="px-3 py-2 text-left">Status</th>
              </tr>
            </thead>
            <tbody>
              {#each preview as row, i (i)}
                {@const hasError = validationErrors.has(i)}
                <tr
                  class="border-t border-gray-100 {hasError ? 'bg-red-50' : ''}"
                >
                  <td class="px-3 py-2 text-gray-400">{i + 1}</td>
                  <td
                    class="px-3 py-2 max-w-xs truncate font-mono text-xs {hasError
                      ? 'text-red-700'
                      : 'text-gray-800'}"
                  >
                    {row.destination_url || "—"}
                  </td>
                  {#if columnMap.short_code && isProOrAbove}
                    <td class="px-3 py-2 font-mono text-xs">
                      <span class="text-gray-600"
                        >{row.short_code || "(auto)"}</span
                      >
                      {#if row.wasTruncated}
                        <span
                          class="ml-1 text-amber-600 text-[10px] font-sans font-medium"
                          >(truncated to 100)</span
                        >
                      {/if}
                    </td>
                  {/if}
                  {#if columnMap.title}
                    <td class="px-3 py-2 text-gray-600 truncate max-w-[150px]"
                      >{row.title || "—"}</td
                    >
                  {/if}
                  <td class="px-3 py-2">
                    {#if hasError}
                      <span class="text-xs text-red-600 font-medium"
                        >{validationErrors.get(i)}</span
                      >
                    {:else}
                      <span class="text-xs text-green-600 font-medium"
                        >✓ OK</span
                      >
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <div class="flex items-center justify-between mt-6">
          <button
            onclick={() => (step = 2)}
            class="text-sm text-gray-500 hover:text-gray-700"
          >
            ← Back
          </button>
          <button
            onclick={goToSettings}
            class="bg-gradient-to-r from-orange-500 to-orange-600 text-white px-6 py-2 rounded-lg font-semibold text-sm hover:shadow-md transition-all"
          >
            Continue →
          </button>
        </div>
      </div>
    {/if}

    <!-- ── Step 4: Import Settings ── -->
    {#if step === 4}
      <div class="bg-white rounded-2xl border border-gray-200 p-8">
        <h2 class="text-lg font-semibold text-gray-900 mb-1">
          Import Settings
        </h2>

        <div class="space-y-4 mt-6">
          <!-- Summary card -->
          <div class="bg-gray-50 rounded-xl p-4 text-sm space-y-1">
            <div class="flex justify-between">
              <span class="text-gray-500">Total rows in file</span>
              <span class="font-semibold text-gray-900">{rawRows.length}</span>
            </div>
            {#if validationErrors.size > 0}
              <div class="flex justify-between">
                <span class="text-gray-500">Rows with invalid URLs</span>
                <span class="font-semibold text-red-600"
                  >{validationErrors.size}</span
                >
              </div>
            {/if}
            <div class="flex justify-between">
              <span class="text-gray-500">Rows to import</span>
              <span class="font-semibold text-orange-600"
                >{validRows.length}</span
              >
            </div>
            {#if remainingQuota !== null}
              <div class="flex justify-between">
                <span class="text-gray-500">Remaining monthly quota</span>
                <span
                  class="font-semibold {validRows.length > remainingQuota
                    ? 'text-red-600'
                    : 'text-green-600'}"
                >
                  {remainingQuota}
                </span>
              </div>
            {/if}
          </div>

          <!-- Skip invalid rows toggle -->
          {#if validationErrors.size > 0}
            <div class="flex items-center gap-3">
              <button
                role="switch"
                aria-checked={skipInvalidRows}
                aria-label="Skip rows with invalid URLs"
                class="relative w-10 h-6 rounded-full transition-colors shrink-0 {skipInvalidRows
                  ? 'bg-orange-500'
                  : 'bg-gray-300'}"
                onclick={() => (skipInvalidRows = !skipInvalidRows)}
              >
                <span
                  class="absolute top-1 left-1 w-4 h-4 bg-white rounded-full shadow transition-transform {skipInvalidRows
                    ? 'translate-x-4'
                    : ''}"
                ></span>
              </button>
              <div>
                <p class="text-sm font-medium text-gray-800">
                  Skip rows with invalid URLs
                </p>
                <p class="text-xs text-gray-500">
                  {skipInvalidRows
                    ? "Invalid rows will be skipped — only valid rows will be imported."
                    : "All rows will be sent; the server will report errors per-row."}
                </p>
              </div>
            </div>
          {/if}

          {#if !isProOrAbove && columnMap.short_code}
            <div
              class="bg-amber-50 border border-amber-200 rounded-xl p-3 text-sm text-amber-700"
            >
              <strong>Free tier:</strong> Custom short codes are not available
              on the Free plan. All links will be assigned auto-generated short
              codes.
              <a href="/pricing" class="underline font-medium">Upgrade to Pro</a
              > to preserve your short codes.
            </div>
          {/if}

          {#if remainingQuota !== null && validRows.length > remainingQuota}
            <div
              class="bg-red-50 border border-red-200 rounded-xl p-3 text-sm text-red-700"
            >
              <strong>Quota warning:</strong> You have {remainingQuota}
              links remaining this month, but are importing {validRows.length}.
              Links beyond your quota will fail.
              <a href="/pricing" class="underline font-medium">Upgrade</a> for a higher
              limit.
            </div>
          {/if}
        </div>

        <div class="flex items-center justify-between mt-8">
          <button
            onclick={() => (step = 3)}
            class="text-sm text-gray-500 hover:text-gray-700"
          >
            ← Back
          </button>
          <button
            onclick={startImport}
            disabled={validRows.length === 0}
            class="bg-gradient-to-r from-orange-500 to-orange-600 text-white px-6 py-2 rounded-lg font-semibold text-sm hover:shadow-md transition-all disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Import {validRows.length} links →
          </button>
        </div>
      </div>
    {/if}

    <!-- ── Step 5: Progress + Results ── -->
    {#if step === 5}
      <div class="bg-white rounded-2xl border border-gray-200 p-8">
        {#if !importDone}
          <h2 class="text-lg font-semibold text-gray-900 mb-1">Importing…</h2>
          <p class="text-sm text-gray-500 mb-6">
            Please wait. Don't close this tab.
          </p>
          <div
            class="w-full bg-gray-200 rounded-full h-3 mb-2 progress-animated"
          >
            <div
              class="bg-gradient-to-r from-orange-500 to-orange-600 h-3 rounded-full transition-all duration-300"
              style="width: {progress}%"
            ></div>
          </div>
          <p class="text-sm text-gray-500 text-right">{progress}%</p>
        {:else}
          <h2 class="text-lg font-semibold text-gray-900 mb-4">
            Import Complete
          </h2>

          <!-- Results summary -->
          <div class="grid grid-cols-3 gap-4 mb-4">
            <div class="bg-green-50 rounded-xl p-4 text-center">
              <p class="text-2xl font-bold text-green-700">
                {totalCreated}
              </p>
              <p class="text-sm text-green-600 mt-1">Created</p>
            </div>
            <div class="bg-amber-50 rounded-xl p-4 text-center">
              <p class="text-2xl font-bold text-amber-700">
                {totalSkipped}
              </p>
              <p class="text-sm text-amber-600 mt-1">Skipped</p>
            </div>
            <div class="bg-red-50 rounded-xl p-4 text-center">
              <p class="text-2xl font-bold text-red-700">
                {totalFailed}
              </p>
              <p class="text-sm text-red-600 mt-1">Failed</p>
            </div>
          </div>

          {#if truncatedCount > 0}
            <div
              class="mb-6 px-4 py-3 bg-amber-50 border border-amber-200 rounded-xl text-sm text-amber-700"
            >
              <strong>{truncatedCount}</strong> short code{truncatedCount === 1
                ? " was"
                : "s were"} longer than 100 characters and {truncatedCount === 1
                ? "was"
                : "were"} automatically truncated before import.
            </div>
          {/if}

          {#if allWarnings.length > 0}
            <div class="mb-4">
              <h3 class="text-sm font-semibold text-amber-700 mb-2">
                Warnings ({allWarnings.length})
              </h3>
              <div
                class="max-h-40 overflow-y-auto rounded-xl border border-amber-100 divide-y divide-amber-100"
              >
                {#each allWarnings as warn, warnIdx (warnIdx)}
                  <div class="px-3 py-2 text-xs flex gap-2">
                    <span class="text-gray-400 shrink-0">Row {warn.row}</span>
                    <span class="font-mono text-gray-600 truncate"
                      >{warn.destination_url}</span
                    >
                    <span class="text-amber-600 shrink-0">{warn.reason}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if allErrors.length > 0}
            <div class="mb-6">
              <div class="flex items-center justify-between mb-2">
                <h3 class="text-sm font-semibold text-gray-800">
                  Errors ({allErrors.length})
                </h3>
                <button
                  onclick={downloadErrorReport}
                  class="text-xs text-orange-600 hover:text-orange-700 font-medium flex items-center gap-1"
                >
                  <svg
                    class="w-3.5 h-3.5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                    />
                  </svg>
                  Download error report
                </button>
              </div>
              <div
                class="max-h-48 overflow-y-auto rounded-xl border border-red-100 divide-y divide-red-100"
              >
                {#each allErrors.slice(0, 50) as err, errIdx (errIdx)}
                  <div class="px-3 py-2 text-xs flex gap-2">
                    <span class="text-gray-400 shrink-0">Row {err.row}</span>
                    <span class="font-mono text-gray-600 truncate"
                      >{err.destination_url}</span
                    >
                    <span class="text-red-600 shrink-0">{err.reason}</span>
                  </div>
                {/each}
                {#if allErrors.length > 50}
                  <div class="px-3 py-2 text-xs text-gray-400">
                    …and {allErrors.length - 50} more. Download the full report above.
                  </div>
                {/if}
              </div>
            </div>
          {/if}

          <div class="flex gap-3">
            <button
              onclick={() => goto("/dashboard")}
              class="flex-1 bg-gradient-to-r from-orange-500 to-orange-600 text-white px-6 py-2.5 rounded-lg font-semibold text-sm hover:shadow-md transition-all text-center"
            >
              Go to dashboard
            </button>
            {#if totalCreated < validRows.length}
              <button
                onclick={() => {
                  step = 1;
                  rawRows = [];
                  csvHeaders = [];
                  progress = 0;
                  importDone = false;
                  totalCreated = 0;
                }}
                class="px-6 py-2.5 rounded-lg border border-gray-200 text-sm font-semibold text-gray-700 hover:bg-gray-50 transition-colors"
              >
                Import more
              </button>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </main>
</div>
