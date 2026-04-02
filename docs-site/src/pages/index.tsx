import type {ReactNode} from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import Heading from '@theme/Heading';

import styles from './index.module.css';

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={clsx('hero hero--primary', styles.heroBanner)}>
      <div className="container">
        <Heading as="h1" className="hero__title">
          {siteConfig.title}
        </Heading>
        <p className="hero__subtitle">{siteConfig.tagline}</p>
        <div className={styles.buttons}>
          <Link
            className="button button--secondary button--lg"
            to="/docs/intro">
            Get Started
          </Link>
          <Link
            className="button button--outline button--secondary button--lg"
            to="/docs/api/rushomon-url-shortener-api">
            API Reference
          </Link>
        </div>
      </div>
    </header>
  );
}

const features = [
  {
    title: 'Interactive API Explorer',
    description:
      'Try API calls directly from the documentation. Authenticate with your API key and explore all endpoints without leaving the browser.',
  },
  {
    title: 'OpenAPI 3.1 Spec',
    description:
      'Every endpoint is documented with full request/response schemas, authentication requirements, and example values.',
  },
  {
    title: 'Versioned Reference',
    description:
      'Browse documentation for any released version. The latest main branch is always available as a preview of upcoming changes.',
  },
];

export default function Home(): ReactNode {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout
      title={siteConfig.title}
      description="API reference and documentation for the Rushomon self-hosted URL shortener.">
      <HomepageHeader />
      <main>
        <section className={styles.features}>
          <div className="container">
            <div className="row">
              {features.map(({title, description}) => (
                <div key={title} className={clsx('col col--4')}>
                  <div className="text--center padding-horiz--md padding-vert--lg">
                    <Heading as="h3">{title}</Heading>
                    <p>{description}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>
      </main>
    </Layout>
  );
}
