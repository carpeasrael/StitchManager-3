import { Marked } from "marked";

const md = new Marked({ gfm: true, breaks: false });

export function renderMarkdown(src: string): string {
  const html = md.parse(src, { async: false });
  if (typeof html !== "string") {
    throw new Error("marked returned a Promise despite async:false");
  }
  return html;
}
