export interface PublicPage {
  path: string;
  changefreq: string;
  priority: string;
  indexable?: boolean;
}

export const PUBLIC_PAGES: PublicPage[] = [
  { path: "/", changefreq: "weekly", priority: "1.0", indexable: true },
  { path: "/pricing", changefreq: "monthly", priority: "0.8", indexable: true },
  {
    path: "/alternatives/bitly",
    changefreq: "monthly",
    priority: "0.8",
    indexable: true
  },
  {
    path: "/alternatives/dub",
    changefreq: "monthly",
    priority: "0.7",
    indexable: true
  },
  {
    path: "/alternatives/short-io",
    changefreq: "monthly",
    priority: "0.7",
    indexable: true
  },
  {
    path: "/compare/rushomon-vs-bitly",
    changefreq: "monthly",
    priority: "0.7",
    indexable: true
  },
  {
    path: "/compare/rushomon-vs-dub",
    changefreq: "monthly",
    priority: "0.7",
    indexable: true
  },
  {
    path: "/compare/rushomon-vs-short-io",
    changefreq: "monthly",
    priority: "0.7",
    indexable: true
  },
  {
    path: "/use-cases/self-hosted",
    changefreq: "monthly",
    priority: "0.7",
    indexable: true
  },
  {
    path: "/use-cases/open-source",
    changefreq: "monthly",
    priority: "0.7",
    indexable: true
  },
  {
    path: "/use-cases/teams",
    changefreq: "monthly",
    priority: "0.6",
    indexable: true
  },
  { path: "/report", changefreq: "monthly", priority: "0.6", indexable: true },
  { path: "/terms", changefreq: "yearly", priority: "0.3", indexable: true },
  { path: "/privacy", changefreq: "yearly", priority: "0.3", indexable: true }
];
