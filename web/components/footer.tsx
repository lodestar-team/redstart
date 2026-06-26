import Link from "next/link";
import { Star } from "./logo";

const REPO = "https://github.com/lodestar-team/redstart";
const DOCS = "https://lodestar-team.github.io/redstart/";

export function Footer() {
  return (
    <footer className="border-t border-line">
      <div className="mx-auto grid max-w-6xl gap-8 px-5 py-14 sm:grid-cols-[1.5fr_1fr_1fr]">
        <div>
          <span className="inline-flex items-center gap-2 text-[1.05rem]">
            <Star className="h-[1.05em] w-[1.05em] text-red" />
            <span className="font-medium tracking-tight">Redstart</span>
          </span>
          <p className="mt-3 max-w-xs text-sm leading-relaxed text-muted">
            One typed language for The Graph subgraphs. A Graph-Foundation-grant
            public good, in the lineage of Matchstick.
          </p>
        </div>
        <div className="flex flex-col gap-2 text-sm text-muted">
          <span className="eyebrow mb-1">Product</span>
          <Link href="/playground" className="transition-colors hover:text-ink">
            Playground
          </Link>
          <a href={DOCS} className="transition-colors hover:text-ink">
            Documentation
          </a>
        </div>
        <div className="flex flex-col gap-2 text-sm text-muted">
          <span className="eyebrow mb-1">Source</span>
          <a href={REPO} className="transition-colors hover:text-ink">
            GitHub
          </a>
          <a href={`${REPO}/releases`} className="transition-colors hover:text-ink">
            Releases
          </a>
          <a href={`${REPO}/tree/main/rfcs`} className="transition-colors hover:text-ink">
            RFCs
          </a>
        </div>
      </div>
      <div className="border-t border-line/70">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-5 py-4 text-xs text-faint">
          <span>MIT licensed · The Lodestar Team</span>
          <span className="font-mono">redstart-lang.com</span>
        </div>
      </div>
    </footer>
  );
}
