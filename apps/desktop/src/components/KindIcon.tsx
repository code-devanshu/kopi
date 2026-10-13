import { Code, File, FileText, Image, Link2, Palette, Type, type LucideIcon } from "lucide-react";
import type { ClipKind } from "../lib/ipc";

const ICONS: Record<ClipKind, LucideIcon> = {
  text: Type,
  rich_text: FileText,
  link: Link2,
  image: Image,
  file: File,
  color: Palette,
  code: Code,
};

export function KindIcon({ kind, size = 15 }: { kind: ClipKind; size?: number }) {
  const Icon = ICONS[kind];
  return <Icon size={size} strokeWidth={1.75} aria-hidden="true" />;
}
