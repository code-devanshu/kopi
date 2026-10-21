import { Pin, TextQuote } from "lucide-react";
import type { ReactNode } from "react";

function MockPanel({ board, snippets, children }: { board: string; snippets?: boolean; children: ReactNode }) {
  const Icon = snippets ? TextQuote : Pin;
  return (
    <div aria-hidden="true" className="rounded-2xl border border-border bg-surface p-4 shadow-lifted">
      <div className="flex items-center justify-between border-b border-border pb-3 text-xs">
        <span className="inline-flex items-center gap-1.5 font-medium">
          <Icon className="size-3.5 text-accent-text" />
          {board}
        </span>
        <span className="text-muted-foreground">{snippets ? "Snippets" : "Pinboard"}</span>
      </div>
      <div className="mt-3 space-y-2">{children}</div>
    </div>
  );
}

function Row({ children }: { children: ReactNode }) {
  return <div className="rounded-lg border border-border bg-background px-3 py-2.5 text-xs">{children}</div>;
}

function SnippetName({ name, keyword }: { name: string; keyword?: string }) {
  return (
    <span className="mb-1 flex items-center gap-1.5 font-medium">
      {name}
      {keyword ? (
        <span className="rounded bg-surface-muted px-1 font-mono text-[10px] text-muted-foreground">{keyword}</span>
      ) : null}
    </span>
  );
}

function Placeholder({ children }: { children: ReactNode }) {
  return <span className="rounded bg-accent-soft px-1 font-mono text-[11px] text-accent-text">{children}</span>;
}

export function DeveloperMock() {
  return (
    <MockPanel board="Commands">
      <Row>
        <code className="font-mono text-[11px]">docker compose up -d --build</code>
      </Row>
      <Row>
        <code className="font-mono text-[11px]">
          <span className="text-accent-text">curl</span> -H &quot;Authorization: Bearer $TOKEN&quot;
        </code>
      </Row>
      <Row>
        <code className="font-mono text-[11px]">SELECT * FROM events WHERE created_at &gt; now() - interval &apos;1 day&apos;;</code>
      </Row>
    </MockPanel>
  );
}

export function DesignerMock() {
  const swatches = ["#2b1d14", "#93561a", "#d08a3a", "#f6e4cc", "#faf6ef"];
  return (
    <MockPanel board="Brand colors">
      <div className="grid grid-cols-5 gap-2">
        {swatches.map((hex) => (
          <div key={hex} className="space-y-1.5">
            <div className="aspect-square rounded-lg ring-1 ring-black/5" style={{ backgroundColor: hex }} />
            <p className="text-center font-mono text-[9px] text-muted-foreground uppercase sm:text-[10px]">{hex}</p>
          </div>
        ))}
      </div>
      <Row>Spacing scale: 4 · 8 · 12 · 16 · 24 · 32 · 48</Row>
      <Row>Corner radius: cards 16px, buttons pill</Row>
    </MockPanel>
  );
}

export function WriterMock() {
  return (
    <MockPanel board="Replies and bios" snippets>
      <Row>
        <SnippetName name="Reply" keyword="ty" />
        Thanks for reaching out, <Placeholder>{"{input:Name}"}</Placeholder>. I&apos;ll get back to you by Friday.
      </Row>
      <Row>
        <SnippetName name="Bio (short)" keyword="bio" />
        Writer and editor covering design and the people behind it.
      </Row>
      <Row>
        <SnippetName name="Dateline" />
        Updated <Placeholder>{"{date}"}</Placeholder>
      </Row>
    </MockPanel>
  );
}

export function StudentMock() {
  return (
    <MockPanel board="Thesis research">
      <Row>
        <span className="text-muted-foreground">Citation · </span>
        Author, A. (2024). Title of the paper. Journal Name, 12(3), 45-67.
      </Row>
      <Row>
        <span className="text-muted-foreground">Link · </span>
        example.edu/library/reading-list
      </Row>
      <Row>
        <span className="text-muted-foreground">Formula · </span>
        <span className="font-mono">E = mc²</span>
      </Row>
    </MockPanel>
  );
}
