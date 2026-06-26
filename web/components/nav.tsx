import Link from "next/link";
import { Wordmark } from "./logo";

const REPO = "https://github.com/lodestar-team/redstart";
const DOCS = "https://lodestar-team.github.io/redstart/";

export function Nav() {
  return (
    <header className="sticky top-0 z-50 border-b border-line/70 bg-canvas/80 backdrop-blur-md">
      <div className="mx-auto flex h-14 max-w-6xl items-center justify-between px-5">
        <Link href="/" className="text-[1.05rem]">
          <Wordmark />
        </Link>
        <nav className="flex items-center gap-1 text-sm text-muted">
          <Link
            href="/playground"
            className="rounded-md px-3 py-1.5 transition-colors hover:bg-surface hover:text-ink"
          >
            Playground
          </Link>
          <a
            href={DOCS}
            className="rounded-md px-3 py-1.5 transition-colors hover:bg-surface hover:text-ink"
          >
            Docs
          </a>
          <a
            href={REPO}
            className="rounded-md px-3 py-1.5 transition-colors hover:bg-surface hover:text-ink"
          >
            GitHub
          </a>
        </nav>
      </div>
    </header>
  );
}
