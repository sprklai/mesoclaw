/**
 * CronBuilder — visual 5-field cron expression builder.
 *
 * Each field (minute, hour, day-of-month, month, day-of-week) can be set via
 * a select or left as `*` (any).  A human-readable preview is rendered below.
 */

import { cn } from "@/lib/utils";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface CronFields {
  minute: string;   // "0"–"59" or "*"
  hour: string;     // "0"–"23" or "*"
  dom: string;      // "1"–"31" or "*"
  month: string;    // "1"–"12" or "*"
  dow: string;      // "0"–"6" or "*"  (0=Sun)
}

/** Parse a 5-field cron expression into structured fields. */
export function parseCron(expr: string): CronFields {
  const parts = expr.trim().split(/\s+/);
  const get = (i: number) => parts[i] ?? "*";
  return { minute: get(0), hour: get(1), dom: get(2), month: get(3), dow: get(4) };
}

/** Serialise fields back to a cron expression string. */
export function buildCron(fields: CronFields): string {
  return `${fields.minute} ${fields.hour} ${fields.dom} ${fields.month} ${fields.dow}`;
}

// ─── Human-readable summary ───────────────────────────────────────────────────

const DOW_NAME = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTH_NAME = ["", "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December"];

function humanReadable(f: CronFields): string {
  const hour =
    f.hour === "*"
      ? "every hour"
      : `${String(Number(f.hour)).padStart(2, "0")}:${f.minute === "*" ? "00" : f.minute.padStart(2, "0")}`;
  const dom = f.dom === "*" ? "" : ` on day ${f.dom}`;
  const month = f.month === "*" ? "" : ` in ${MONTH_NAME[Number(f.month)] ?? f.month}`;
  const dow =
    f.dow === "*"
      ? ""
      : ` on ${DOW_NAME[Number(f.dow)] ?? `weekday ${f.dow}`}`;

  if (f.hour === "*" && f.minute === "*") return `Every minute${dom}${month}${dow}`;
  if (f.minute === "*") return `Every minute of hour ${f.hour}${dom}${month}${dow}`;
  return `At ${hour}${dom}${month}${dow}`;
}

// ─── FieldSelect ──────────────────────────────────────────────────────────────

interface FieldSelectProps {
  label: string;
  value: string;
  options: Array<{ value: string; label: string }>;
  onChange: (v: string) => void;
}

function FieldSelect({ label, value, options, onChange }: FieldSelectProps) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {label}
      </label>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="rounded-md border bg-background px-2 py-1 text-xs focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </div>
  );
}

// ─── Option builders ──────────────────────────────────────────────────────────

const ANY = { value: "*", label: "* (any)" };

function rangeOptions(from: number, to: number, labelFn?: (n: number) => string) {
  return [ANY, ...Array.from({ length: to - from + 1 }, (_, i) => {
    const n = from + i;
    return { value: String(n), label: labelFn ? labelFn(n) : String(n) };
  })];
}

const MINUTE_OPTS = rangeOptions(0, 59);
const HOUR_OPTS = rangeOptions(0, 23, (n) => `${String(n).padStart(2, "0")}:00`);
const DOM_OPTS = rangeOptions(1, 31);
const MONTH_OPTS = rangeOptions(1, 12, (n) => `${n} – ${MONTH_NAME[n]}`);
const DOW_OPTS = [ANY, ...DOW_NAME.map((name, i) => ({ value: String(i), label: name }))];

// ─── CronBuilder ─────────────────────────────────────────────────────────────

interface CronBuilderProps {
  value: string;
  onChange: (expr: string) => void;
  className?: string;
}

export function CronBuilder({ value, onChange, className }: CronBuilderProps) {
  const fields = parseCron(value);

  const update = (patch: Partial<CronFields>) => {
    onChange(buildCron({ ...fields, ...patch }));
  };

  return (
    <div className={cn("flex flex-col gap-3", className)}>
      {/* Fields row */}
      <div className="grid grid-cols-5 gap-2">
        <FieldSelect
          label="Minute"
          value={fields.minute}
          options={MINUTE_OPTS}
          onChange={(v) => update({ minute: v })}
        />
        <FieldSelect
          label="Hour"
          value={fields.hour}
          options={HOUR_OPTS}
          onChange={(v) => update({ hour: v })}
        />
        <FieldSelect
          label="Day"
          value={fields.dom}
          options={DOM_OPTS}
          onChange={(v) => update({ dom: v })}
        />
        <FieldSelect
          label="Month"
          value={fields.month}
          options={MONTH_OPTS}
          onChange={(v) => update({ month: v })}
        />
        <FieldSelect
          label="Weekday"
          value={fields.dow}
          options={DOW_OPTS}
          onChange={(v) => update({ dow: v })}
        />
      </div>

      {/* Raw expression */}
      <div className="flex items-center gap-2">
        <span className="text-xs text-muted-foreground">Expression:</span>
        <code className="rounded bg-muted px-2 py-0.5 font-mono text-xs">
          {value}
        </code>
      </div>

      {/* Human-readable preview */}
      <p className="text-xs text-muted-foreground italic">
        {humanReadable(fields)}
      </p>
    </div>
  );
}
