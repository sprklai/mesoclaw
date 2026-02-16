import { Label } from "@/components/ui/label";

interface SettingRowProps {
  label: string;
  description: string;
  htmlFor?: string;
  children: React.ReactNode;
}

export function SettingRow({
  label,
  description,
  htmlFor,
  children,
}: SettingRowProps) {
  return (
    <div className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
      <div className="min-w-0 space-y-0.5">
        <Label htmlFor={htmlFor} className="text-base font-medium">
          {label}
        </Label>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
      <div className="shrink-0 self-start sm:self-center">{children}</div>
    </div>
  );
}
