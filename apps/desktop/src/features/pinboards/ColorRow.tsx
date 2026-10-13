import { Ban } from "lucide-react";
import { COLORS } from "./boards";

/** A pinboard's colors as swatches, with one for no color, for its menu. */
export function ColorRow({ value, onPick }: { value: string | null; onPick: (color: string | null) => void }) {
  return (
    <div className="color-row" role="group" aria-label="Color">
      {COLORS.map(({ name, hex }) => (
        <button
          key={hex}
          type="button"
          className="swatch-button"
          title={name}
          aria-label={name}
          aria-pressed={value === hex}
          tabIndex={-1}
          style={{ background: hex }}
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => onPick(hex)}
        />
      ))}
      <button
        type="button"
        className="swatch-button none"
        title="No color"
        aria-label="No color"
        aria-pressed={value === null}
        tabIndex={-1}
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => onPick(null)}
      >
        <Ban size={12} strokeWidth={2} aria-hidden="true" />
      </button>
    </div>
  );
}
