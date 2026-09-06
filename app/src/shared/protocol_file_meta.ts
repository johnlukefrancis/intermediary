// Path: app/src/shared/protocol_file_meta.ts
// Description: Leaf file metadata enums shared by the file change and file delta event schemas

import { z } from "zod";

/** Classification the agent assigns to a watched path for column routing */
export const FileKindSchema = z.enum(["docs", "code", "image", "other"]);
export type FileKind = z.infer<typeof FileKindSchema>;

/** Raw watcher change type reported per notify event */
export const FileChangeTypeSchema = z.enum(["add", "change", "unlink"]);
export type FileChangeType = z.infer<typeof FileChangeTypeSchema>;
