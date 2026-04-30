export interface CompetitorFeature {
  feature: string;
  rushomon: string | boolean;
  competitor: string | boolean;
}

export interface Competitor {
  slug: string;
  name: string;
  tagline: string;
  heroHeading: string;
  heroSubheading: string;
  metaTitle: string;
  metaDescription: string;
  mainPitch: string;
  features: CompetitorFeature[];
  whyRushomon: { title: string; body: string }[];
}

export const COMPETITORS: Record<string, Competitor> = {
  bitly: {
    slug: "bitly",
    name: "Bitly",
    tagline: "The most popular link shortener",
    heroHeading: "The Open Source Bitly Alternative",
    heroSubheading:
      "Everything Bitly offers — click analytics, branded links — without the $299/month price tag or the data lock-in.",
    metaTitle: "Bitly Alternative — Open Source URL Shortener | Rushomon",
    metaDescription:
      "Looking for a Bitly alternative? Rushomon is a free, open source URL shortener with analytics and self-hosting. No lock-in, your data.",
    mainPitch:
      "Bitly is the category leader, but its pricing starts at $8/month and quickly escalates to $199–$299/month for teams wanting analytics history. Rushomon gives you the same core capabilities — short links, click analytics — on a generous free tier, with no lock-in because it's open source and self-hostable.",
    features: [
      {
        feature: "Free tier",
        rushomon: "Yes — 15 links/month",
        competitor: "Yes — 5 links/month"
      },
      {
        feature: "Custom short codes",
        rushomon: "Yes (Pro)",
        competitor: "Yes (paid)"
      },
      {
        feature: "Click analytics",
        rushomon: "Yes",
        competitor: "Yes"
      },
      {
        feature: "Analytics retention (free)",
        rushomon: "7 days",
        competitor: "None — paid plans only"
      },
      {
        feature: "Open source",
        rushomon: "Yes (AGPL-3.0)",
        competitor: "No"
      },
      {
        feature: "Self-hostable",
        rushomon: "Yes — Cloudflare Workers",
        competitor: "No"
      },
      {
        feature: "Data ownership",
        rushomon: "Full ownership",
        competitor: "Bitly owns your data"
      },
      {
        feature: "API access",
        rushomon: "Yes (Pro)",
        competitor: "Yes (paid)"
      },
      {
        feature: "Team features",
        rushomon: "Yes (Business)",
        competitor: "Yes (Enterprise)"
      }
    ],
    whyRushomon: [
      {
        title: "No data lock-in",
        body: "Your links and analytics data belong to you. Export at any time, or self-host for complete control."
      },
      {
        title: "Transparent pricing",
        body: "Free tier for individuals, affordable Pro and Business plans. No surprise bills or feature gates for basic analytics."
      },
      {
        title: "Open source",
        body: "Built with Rust and SvelteKit under AGPL-3.0. Audit the code, fork it, or contribute improvements."
      }
    ]
  },
  dub: {
    slug: "dub",
    name: "Dub.co",
    tagline: "The modern developer-focused link shortener",
    heroHeading: "Self-Host Instead of Dub.co",
    heroSubheading:
      "Dub is great, but at $24–$99/month it adds up. Rushomon is open source and self-hostable on Cloudflare Workers — one-time setup, zero monthly SaaS fees.",
    metaTitle: "Dub Alternative — Self-Hosted URL Shortener | Rushomon",
    metaDescription:
      "Looking for a Dub.co alternative you can self-host? Rushomon runs on Cloudflare Workers, is fully open source, and has a generous free tier.",
    mainPitch:
      "Dub.co is a polished modern tool beloved by developers. But its free plan caps you at 1,000 tracked events per month — once your links get any real traffic, you're forced onto a paid tier. Rushomon tracks unlimited clicks with no event caps. And if you're comfortable with Cloudflare Workers, self-hosting gives you the full feature set with zero monthly fees and no third-party dependency.",
    features: [
      {
        feature: "Free tier",
        rushomon: "Yes — 15 links/month",
        competitor: "Yes — 25 links/month"
      },
      {
        feature: "Analytics retention (free)",
        rushomon: "7 days",
        competitor: "30 days"
      },
      {
        feature: "Custom short codes",
        rushomon: "Yes (Pro)",
        competitor: "Yes (paid)"
      },
      {
        feature: "Click tracking",
        rushomon: "Unlimited clicks",
        competitor: "1K events/month (free)"
      },
      {
        feature: "Click analytics",
        rushomon: "Yes",
        competitor: "Yes"
      },
      {
        feature: "Open source",
        rushomon: "Yes (AGPL-3.0)",
        competitor: "Yes (AGPLv3)"
      },
      {
        feature: "Self-hostable",
        rushomon: "Yes — Cloudflare Workers",
        competitor: "Yes — complex infra"
      },
      {
        feature: "Cloudflare-native",
        rushomon: "Yes — built for Workers + KV + D1",
        competitor: "No — requires own server"
      },
      {
        feature: "API access",
        rushomon: "Yes (Pro)",
        competitor: "Yes (free)"
      },
      {
        feature: "Team features",
        rushomon: "Yes (Business)",
        competitor: "Yes (paid)"
      },
      {
        feature: "Password protection",
        rushomon: "Yes (Business)",
        competitor: "Yes (paid)"
      }
    ],
    whyRushomon: [
      {
        title: "Cloudflare-native by design",
        body: "Rushomon is built specifically for Cloudflare Workers, KV, and D1. Sub-millisecond redirects at the edge globally — not an afterthought."
      },
      {
        title: "Zero-dependency self-hosting",
        body: "No servers, no Docker, no databases to manage. One wrangler deploy command and you're running your own link shortener at the edge."
      },
      {
        title: "Familiar stack",
        body: "Rust backend compiling to WebAssembly, SvelteKit frontend. Modern, performant, and easy to contribute to."
      }
    ]
  },
  "short-io": {
    slug: "short-io",
    name: "Short.io",
    tagline: "A popular link shortener for teams",
    heroHeading: "Free Short.io Alternative with Full Data Ownership",
    heroSubheading:
      "Short.io caps free users at 1,000 total links and hides analytics after 50K clicks. Rushomon is open source, self-hostable, and never gates your own data.",
    metaTitle:
      "Short.io Alternative — Free, Open Source URL Shortener | Rushomon",
    metaDescription:
      "Looking for a Short.io alternative? Rushomon is free, open source, and self-hostable. Get click analytics and team support without $19/month.",
    mainPitch:
      "Short.io is a solid commercial URL shortener, but its free plan caps you at 1,000 total links ever and stops showing you analytics after 50,000 clicks per month — the data keeps being collected, you just can't see it without upgrading. It's also fully closed-source, so you're entirely dependent on their servers and business decisions. Rushomon is open source under AGPL-3.0, has no analytics visibility gates, and can be self-hosted on Cloudflare Workers if you want full control.",
    features: [
      {
        feature: "Free tier",
        rushomon: "Yes — 15 links/month",
        competitor: "Yes — 1,000 links total"
      },
      {
        feature: "Click tracking",
        rushomon: "Unlimited clicks",
        competitor: "50K/month (then hidden)"
      },
      {
        feature: "Custom short codes",
        rushomon: "Yes (Pro)",
        competitor: "Yes (paid)"
      },
      {
        feature: "Click analytics",
        rushomon: "Yes",
        competitor: "Yes (capped on free)"
      },
      {
        feature: "Open source",
        rushomon: "Yes (AGPL-3.0)",
        competitor: "No"
      },
      {
        feature: "Self-hostable",
        rushomon: "Yes — Cloudflare Workers",
        competitor: "No"
      },
      {
        feature: "Data ownership",
        rushomon: "Full ownership",
        competitor: "Vendor-controlled"
      },
      {
        feature: "API access",
        rushomon: "Yes (Pro)",
        competitor: "Yes (paid)"
      },
      {
        feature: "Team features",
        rushomon: "Yes (Business)",
        competitor: "Yes (paid)"
      },
      {
        feature: "QR code generation",
        rushomon: "Yes",
        competitor: "Yes"
      }
    ],
    whyRushomon: [
      {
        title: "Your links, your rules",
        body: "Being open source means you can inspect every line of code and self-host if you want zero dependency on Rushomon's servers."
      },
      {
        title: "No vendor lock-in",
        body: "Export your full link library at any time. If Rushomon ever disappeared, you could run the open source code yourself."
      },
      {
        title: "Cloudflare edge performance",
        body: "Links resolve at the nearest Cloudflare data center, giving you sub-millisecond redirects for users anywhere in the world."
      }
    ]
  }
};

export const COMPETITOR_SLUGS = Object.keys(COMPETITORS);
