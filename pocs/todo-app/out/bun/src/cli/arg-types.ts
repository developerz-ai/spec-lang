// Projected by hand from TaskNo, Title, Tag, Priority as command-line values (src/todo/model.slg, docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { PRIORITIES, type Priority } from "../model/priority.ts";
import { asTag, type Tag } from "../model/tag.ts";
import { asTaskNo, type TaskNo } from "../model/task-no.ts";
import { asTitle, type Title } from "../model/title.ts";

/** How one argument becomes a typed value; `undefined` means "not a <name>". */
export interface ArgType<T> {
  readonly name: string;
  /** What `--help` shows after a flag of this type. */
  readonly hint: string;
  readonly parse: (text: string) => T | undefined;
}

const DIGITS = /^[0-9]+$/;

/** ASCII digits only, leading zeros allowed, any length: `Int` is unbounded. */
export const TASK_NO: ArgType<TaskNo> = Object.freeze({
  name: "TaskNo",
  hint: "<no>",
  parse: (text: string) => (DIGITS.test(text) ? asTaskNo(BigInt(text)) : undefined),
});

export const TITLE: ArgType<Title> = Object.freeze({ name: "Title", hint: "<title>", parse: asTitle });

export const TAG: ArgType<Tag> = Object.freeze({ name: "Tag", hint: "<tag>", parse: asTag });

/** A sum type with no data: its variant names in lower case. */
const PRIORITY_BY_NAME: ReadonlyMap<string, Priority> = new Map(PRIORITIES.map((p) => [p.toLowerCase(), p]));

export const PRIORITY: ArgType<Priority> = Object.freeze({
  name: "Priority",
  hint: [...PRIORITY_BY_NAME.keys()].join("|"),
  parse: (text: string) => PRIORITY_BY_NAME.get(text),
});
