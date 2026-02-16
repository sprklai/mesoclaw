import { Mic } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { ComponentProps } from "react";

export interface SpeechInputProps extends Omit<ComponentProps<typeof Button>, "onClick"> {
  onTranscriptionChange?: (transcript: string) => void;
}

export function SpeechInput({
  onTranscriptionChange: _onTranscriptionChange,
  className,
  ...props
}: SpeechInputProps) {
  // Placeholder implementation - speech recognition not implemented yet
  return (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      className={className}
      {...props}
      onClick={() => {
        // TODO: Implement speech recognition
        console.log("Speech input not implemented yet");
      }}
    >
      <Mic className="size-4" />
    </Button>
  );
}
