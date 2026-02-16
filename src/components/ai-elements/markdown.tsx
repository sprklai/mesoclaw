import ReactMarkdown from "react-markdown";

export function Markdown({ content }: { content: string }) {
  return (
    <div className="prose prose-sm max-w-none dark:prose-invert">
      <ReactMarkdown>{content}</ReactMarkdown>
    </div>
  );
}
