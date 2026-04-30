export interface UseCaseFeature {
  title: string;
  body: string;
}

export interface UseCase {
  slug: string;
  heroHeading: string;
  heroSubheading: string;
  metaTitle: string;
  metaDescription: string;
  intro: string;
  features: UseCaseFeature[];
  faqs: { q: string; a: string }[];
}

export const USE_CASES: Record<string, UseCase> = {
  "self-hosted": {
    slug: "self-hosted",
    heroHeading: "Self-Hosted URL Shortener on Cloudflare Workers",
    heroSubheading:
      "Run your own link shortener at the edge. One wrangler deploy, your domain, zero monthly SaaS fees.",
    metaTitle: "Self-Hosted URL Shortener — Cloudflare Workers | Rushomon",
    metaDescription:
      "Rushomon is a self-hosted URL shortener built for Cloudflare Workers. Open source, free to host, sub-millisecond redirects globally. No servers to manage.",
    intro:
      "Most URL shorteners are SaaS products — you pay monthly, you depend on their uptime, and your data lives on their servers. Rushomon takes a different approach: it's open source software you deploy to your own Cloudflare account in minutes. You get global edge performance, full data ownership, and no ongoing SaaS fees.",
    features: [
      {
        title: "Deploys in minutes",
        body: "Clone the repo, set your secrets, run wrangler deploy. Your URL shortener is live at your domain with sub-millisecond global performance."
      },
      {
        title: "Zero infrastructure to manage",
        body: "Runs on Cloudflare Workers (serverless), KV (edge key-value store), and D1 (SQLite at the edge). No servers, no Docker, no databases to maintain."
      },
      {
        title: "Your domain, your brand",
        body: "Use any domain you own as your short link domain. rush.mn, go.yourbrand.com, links.company.io — it's all yours."
      },
      {
        title: "Full data ownership",
        body: "All link data, click analytics, and user accounts live in your Cloudflare account. You can export or delete everything at any time."
      },
      {
        title: "Open source codebase",
        body: "Every line of code is public on GitHub under AGPL-3.0. Audit it, extend it, or contribute improvements."
      },
      {
        title: "Free Cloudflare tier is enough",
        body: "Cloudflare's free plan covers the Workers, KV, and D1 resources needed to run a personal or small-team URL shortener."
      }
    ],
    faqs: [
      {
        q: "Do I need a paid Cloudflare account?",
        a: "No. Rushomon runs within Cloudflare's free tier limits for personal and small team use. A paid Workers plan is only needed for very high traffic volumes."
      },
      {
        q: "How long does setup take?",
        a: "Around 15–30 minutes for a first-time Cloudflare Workers deployment. The SELF_HOSTING.md guide walks through every step."
      },
      {
        q: "What happens if Rushomon the company disappears?",
        a: "Nothing changes for self-hosters. The open source code continues to exist on GitHub, and your deployment keeps running on your Cloudflare account."
      }
    ]
  },
  "open-source": {
    slug: "open-source",
    heroHeading: "Open Source URL Shortener — Audit, Fork, Self-Host",
    heroSubheading:
      "Built with Rust and SvelteKit, licensed under AGPL-3.0. No black boxes, no data-sharing agreements, no vendor dependency.",
    metaTitle: "Open Source URL Shortener — AGPL-3.0 | Rushomon",
    metaDescription:
      "Rushomon is a fully open source URL shortener built with Rust and SvelteKit. Self-host on Cloudflare Workers or use the hosted service. AGPL-3.0 license.",
    intro:
      "Most link shorteners — even ones that claim to care about privacy — are proprietary software running on servers you don't control. Rushomon is different: the entire codebase is public on GitHub, licensed under AGPL-3.0. You can read every line, self-host the whole thing, and never worry about a vendor changing terms or going offline.",
    features: [
      {
        title: "Full source on GitHub",
        body: "The entire Rushomon codebase — Rust backend, SvelteKit frontend, database migrations — is public at github.com/piffio/rushomon."
      },
      {
        title: "AGPL-3.0 license",
        body: "Modifications must also be open source. If you run a modified version as a network service, you must publish your changes — keeping the ecosystem honest."
      },
      {
        title: "Rust + WebAssembly backend",
        body: "The backend runs as a Cloudflare Worker compiled from Rust to WebAssembly. Fast, memory-safe, and auditable."
      },
      {
        title: "SvelteKit frontend",
        body: "The dashboard and landing page are built with SvelteKit and Tailwind CSS. The full frontend source is included."
      },
      {
        title: "Community contributions welcome",
        body: "Open issues, submit PRs, or fork for your own needs. The project follows standard GitHub contribution workflows."
      },
      {
        title: "No telemetry",
        body: "Self-hosted instances send no data back to Rushomon. What happens in your Cloudflare account stays in your Cloudflare account."
      }
    ],
    faqs: [
      {
        q: "Can I use Rushomon for commercial purposes?",
        a: "Yes, including for commercial self-hosted deployments. The AGPL-3.0 license requires you to publish modifications if you offer the software as a network service."
      },
      {
        q: "Is the hosted rushomon.cc service the same code as open source?",
        a: "Yes. rushomon.cc runs the exact same code published on GitHub. No proprietary additions or hidden features."
      },
      {
        q: "Can I fork Rushomon and use a different license?",
        a: "No. AGPL-3.0 is copyleft — forks and derivative works must also use AGPL-3.0 or a compatible license."
      },
      {
        q: "How do I contribute?",
        a: "Open a GitHub issue or pull request at github.com/piffio/rushomon. The CONTRIBUTING.md file has setup instructions."
      }
    ]
  },
  teams: {
    slug: "teams",
    heroHeading: "URL Shortener for Teams — Shared Links, Shared Analytics",
    heroSubheading:
      "One organization, multiple team members, centralized analytics. Keep everyone's short links in one place.",
    metaTitle: "URL Shortener for Teams | Rushomon",
    metaDescription:
      "Rushomon's Business plan lets teams share a link library, view combined analytics, and manage members under one organization. Self-hostable for enterprises.",
    intro:
      "When multiple people on a team are creating short links independently — different tools, different domains, no shared analytics — things get messy fast. Rushomon's organization model puts all team links and analytics in one place, with role-based access, so everyone works from the same source of truth.",
    features: [
      {
        title: "Shared organization",
        body: "Every link belongs to an organization. Team members see and manage the full shared link library, not just their own."
      },
      {
        title: "Combined analytics",
        body: "See aggregated click analytics across all team links. Understand which campaigns, channels, and content types drive the most traffic."
      },
      {
        title: "Role-based access",
        body: "Admins manage members and settings. Regular members create and manage links. Permissions are enforced at the API level."
      },
      {
        title: "Up to 20 team members",
        body: "Business plan supports up to 20 members per organization — enough for marketing teams, agencies, and mid-size companies."
      },
      {
        title: "10,000 links per month",
        body: "Business plan supports 10,000 new links per month — no worrying about hitting limits during big campaigns."
      },
      {
        title: "API access for automation",
        body: "Automate link creation from CI/CD pipelines, marketing tools, or custom scripts using the REST API with Personal Access Tokens."
      }
    ],
    faqs: [
      {
        q: "How many team members can share one organization?",
        a: "Up to 20 members on the Business plan. The Pro plan is optimized for individual use."
      },
      {
        q: "Can team members have different permission levels?",
        a: "Yes. Organization admins can manage members and billing. Regular members can create and manage links."
      },
      {
        q: "Can we self-host for our enterprise?",
        a: "Yes. Rushomon is open source and can be self-hosted on your Cloudflare account. Ideal for enterprises with data residency requirements."
      },
      {
        q: "Is there a limit on how many organizations we can create?",
        a: "Business plan supports up to 3 organizations. Contact us for enterprise needs beyond that."
      }
    ]
  }
};

export const USE_CASE_SLUGS = Object.keys(USE_CASES);
