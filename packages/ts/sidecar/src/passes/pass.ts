import type { Edit } from '#sidecar/syntax/edits';
import type { SourceDocument } from '#sidecar/syntax/source-document';

/** One deterministic formatting rule: reads a document, proposes edits. */
export interface FormattingPass {
	readonly name: string;
	computeEdits(document: SourceDocument): Edit[];
}
