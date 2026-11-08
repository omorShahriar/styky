import { Children, isValidElement, useMemo } from "react";
import type { ComponentProps } from "react";
import ReactMarkdown from "react-markdown";
import type { Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import type { Options as SanitizeSchema } from "rehype-sanitize";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isTaskChecked } from "../lib/markdown";
import { CheckIcon } from "./CheckIcon";

// GitHub-style schema, plus `checked` so task boxes survive sanitising.
const schema: SanitizeSchema = {
  ...defaultSchema,
  attributes: {
    ...defaultSchema.attributes,
    input: [...(defaultSchema.attributes?.input ?? []), "checked"],
  },
};

const remarkPlugins = [remarkGfm];
const rehypePlugins = [[rehypeSanitize, schema]] as ComponentProps<typeof ReactMarkdown>["rehypePlugins"];

interface Props {
  source: string;
  /** Called with the 1-based source line of the task that was toggled. */
  onToggleTask: (line: number) => void;
  onError: (message: string) => void;
}

export function MarkdownView({ source, onToggleTask, onError }: Props) {
  const components = useMemo<Components>(
    () => ({
      a({ href, children }) {
        return (
          <a
            href={href}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              if (href) openUrl(href).catch((err) => onError(`Could not open link: ${String(err)}`));
            }}
          >
            {children}
          </a>
        );
      },
      li({ node, className, children }) {
        const line = node?.position?.start.line;
        if (!className?.includes("task-list-item") || !line) {
          return <li className={className}>{children}</li>;
        }
        const checked = isTaskChecked(source, line) ?? false;
        const kids = Children.map(children, (child) =>
          isValidElement(child) && child.type === "input" ? (
            <button
              type="button"
              role="checkbox"
              aria-checked={checked}
              className={`task-box${checked ? " on" : ""}`}
              onClick={(e) => {
                e.stopPropagation();
                onToggleTask(line);
              }}
            >
              {checked && <CheckIcon size={9} />}
            </button>
          ) : (
            child
          ),
        );
        return <li className={className}>{kids}</li>;
      },
    }),
    [source, onToggleTask, onError],
  );

  return (
    <div className="md">
      <ReactMarkdown remarkPlugins={remarkPlugins} rehypePlugins={rehypePlugins} components={components}>
        {source}
      </ReactMarkdown>
    </div>
  );
}
