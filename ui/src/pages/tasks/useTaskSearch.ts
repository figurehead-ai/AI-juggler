/**
 * The tasks' search box (a pool's Tasks, the Tasks screen): what's typed,
 * and what's looked for: the words, trimmed, once typing pauses (at once
 * when the box is emptied).
 */
import { useEffect, useState } from "react";

export function useTaskSearch(): { words: string; setWords: (next: string) => void; query: string } {
  const [words, setWords] = useState("");
  const [query, setQuery] = useState("");
  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(words.trim()), words ? 200 : 0);
    return () => window.clearTimeout(timer);
  }, [words]);
  return { words, setWords, query };
}
