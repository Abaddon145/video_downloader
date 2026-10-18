import { useMemo, type RefObject } from 'react';
import CodeMirror, { EditorView, type ReactCodeMirrorRef } from '@uiw/react-codemirror';
import { markdown } from '@codemirror/lang-markdown';
import {useWorkspace} from '../workspace/WorkspaceProvider';

type MarkdownEditorProps = {
  value: string;
  onChange: (value: string) => void;
  readOnly?: boolean;
  editorRef: RefObject<ReactCodeMirrorRef | null>;
};

export function MarkdownEditor({ value, onChange, readOnly = false, editorRef }: MarkdownEditorProps) {
  const {resolvedTheme}=useWorkspace();
  const extensions = useMemo(
    () => [
      markdown(),
      EditorView.lineWrapping,
      EditorView.contentAttributes.of({ 'aria-label': 'Markdown正文' }),
    ],
    [],
  );

  return (
    <CodeMirror
      ref={editorRef}
      value={value}
      onChange={onChange}
      extensions={extensions}
      theme={resolvedTheme}
      height="100%"
      minHeight="420px"
      readOnly={readOnly}
      editable={!readOnly}
      indentWithTab={false}
      className="knowledge-code-editor"
    />
  );
}
