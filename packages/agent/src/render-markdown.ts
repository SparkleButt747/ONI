import { marked } from "marked";
import { markedTerminal } from "marked-terminal";

// Configure marked with terminal renderer
marked.use(
  markedTerminal({
    tab: 2,
    emoji: false,
  }) as marked.MarkedExtension,
);

/**
 * Render a markdown string to ANSI-coloured terminal output.
 * Used to format Claude's responses which contain markdown.
 */
export function renderMarkdown(text: string): string {
  try {
    // marked.parse can return string or Promise<string>
    const result = marked.parse(text);
    if (typeof result === "string") {
      return result;
    }
    // Shouldn't happen with sync extensions, but fallback
    return text;
  } catch {
    // If rendering fails, return raw text
    return text;
  }
}

/**
 * Check if text contains markdown formatting worth rendering.
 * Avoids unnecessary processing for plain text responses.
 */
export function hasMarkdown(text: string): boolean {
  return /[*_`#\[\]|>-]/.test(text) && (
    /\*\*[^*]+\*\*/.test(text) ||     // bold
    /`[^`]+`/.test(text) ||            // inline code
    /```[\s\S]+```/.test(text) ||      // code block
    /^#{1,6}\s/m.test(text) ||         // headers
    /^\s*[-*]\s/m.test(text) ||        // lists
    /^\s*\d+\.\s/m.test(text) ||      // numbered lists
    /^\s*>/m.test(text)                // blockquotes
  );
}
