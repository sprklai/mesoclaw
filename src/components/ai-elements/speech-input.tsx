import { Mic } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tooltip } from "@/components/ui/tooltip";
import type { ComponentProps } from "react";

export interface SpeechInputProps extends Omit<ComponentProps<typeof Button>, "onClick"> {
  onTranscriptionChange?: (transcript: string) => void;
}

export function SpeechInput({
  onTranscriptionChange: _onTranscriptionChange,
  className,
  ...props
}: SpeechInputProps) {
  return (
    <Tooltip content="Speech input coming soon" side="top">
      <Button
        type="button"
        variant="ghost"
        size="sm"
        className={className}
        disabled
        {...props}
      >
        <Mic className="size-4" />
      </Button>
    </Tooltip>
  );
}
