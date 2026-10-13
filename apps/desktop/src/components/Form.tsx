import { ChevronDown } from "lucide-react";
import { useId, type ReactNode } from "react";

/** A titled group of rows, as in System Settings. */
export function Group({ title, footer, children }: { title?: string; footer?: ReactNode; children: ReactNode }) {
  return (
    <section className="group">
      {title && <h2 className="group-title">{title}</h2>}
      <div className="group-body">{children}</div>
      {footer && <p className="group-footer">{footer}</p>}
    </section>
  );
}

/** A label and its description on the left, the control on the right. */
export function Row({
  label,
  description,
  htmlFor,
  children,
}: {
  label: ReactNode;
  description?: ReactNode;
  /** The control's id, so clicking the label focuses it. */
  htmlFor?: string;
  children?: ReactNode;
}) {
  return (
    <div className="row-setting">
      <div className="row-setting-text">
        {htmlFor ? (
          <label className="row-setting-label" htmlFor={htmlFor}>
            {label}
          </label>
        ) : (
          <span className="row-setting-label">{label}</span>
        )}
        {description && <p className="row-setting-description">{description}</p>}
      </div>
      {children !== undefined && <div className="row-setting-control">{children}</div>}
    </div>
  );
}

export function Switch({
  id,
  checked,
  label,
  disabled,
  onChange,
}: {
  id?: string;
  checked: boolean;
  /** For screen readers, when no visible label points at it. */
  label?: string;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <button
      id={id}
      type="button"
      role="switch"
      className="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    >
      <span className="switch-thumb" />
    </button>
  );
}

/** A row of buttons, one of them chosen. */
export function Segmented<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          className="segment"
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

/** A native select, styled like the panel's filters. */
export function Select({
  id,
  value,
  options,
  onChange,
  label,
}: {
  id?: string;
  value: string;
  options: { value: string; label: string; group?: string }[];
  onChange: (value: string) => void;
  label?: string;
}) {
  const fallbackId = useId();
  const groups = [...new Set(options.map((o) => o.group))];
  const render = (o: { value: string; label: string }) => (
    <option key={o.value} value={o.value}>
      {o.label}
    </option>
  );
  return (
    <span className="select">
      <select id={id ?? fallbackId} value={value} aria-label={label} onChange={(e) => onChange(e.target.value)}>
        {groups.map((group) =>
          group === undefined ? (
            options.filter((o) => o.group === undefined).map(render)
          ) : (
            <optgroup key={group} label={group}>
              {options.filter((o) => o.group === group).map(render)}
            </optgroup>
          ),
        )}
      </select>
      <ChevronDown className="select-chevron" size={13} strokeWidth={2} aria-hidden="true" />
    </span>
  );
}

/** An inline error, announced when it appears. */
export function FormError({ message }: { message: string | null }) {
  return (
    <p className="form-error" role="alert">
      {message}
    </p>
  );
}
