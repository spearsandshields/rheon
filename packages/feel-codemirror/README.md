# @rheon/feel-codemirror

CodeMirror 6 support for FEEL from the rheon engine's own parser. The colours, the error marks
and the completions are what the engine answers for the text, so the editor reads a text as
the engine evaluates it: a name of several words is one token, and an error mark stands where
`evaluate` would refuse the text.

```ts
import { EditorView, basicSetup } from 'codemirror';
import { FeelEngine } from '@rheon/feel-runtime';
import { feelEditor, setFeelScope } from '@rheon/feel-codemirror';

const engine = await FeelEngine.create({ wasmUrl });
const view = new EditorView({
  parent,
  extensions: [basicSetup, feelEditor({ engine, context: { 'Monthly Salary': 5000 } })],
});

// other names in scope, or unary tests
view.dispatch({ effects: setFeelScope({ dialect: 'unaryTests', outline }) });
```

| Option | Meaning | Default |
|---|---|---|
| `engine` | What answers: a `FeelEngine`, or `instanceSource(instance)` for an in-thread engine | required |
| `dialect`, `context`, `contextFormat`, `outline` | The names in scope, as for the engine's editor operations. With a large context, ask the engine for its `outline` once and give that | expression, no names |
| `maxDiagnostics` | The most syntax errors that are marked; the engine reports up to 100 | 20 |
| `templates` | Offer templates of `if`, `for`, `some`, `every`, a function and a context | true |
| `tree` | Ask for the nodes of the text too (`syntaxOf(state).nodes`) | false |
| `onSyntax` | Called with every answer the editor takes | none |

- `syntaxOf(state)` is the last answer the editor took (tokens, errors, nodes).
- Colours are CSS custom properties with defaults: `--feel-keyword`, `--feel-number`,
  `--feel-string`, `--feel-constant`, `--feel-function`, `--feel-property`, `--feel-name`,
  `--feel-comment`, `--feel-operator`, `--feel-punctuation`, `--feel-error`. The classes are
  `cm-feel-<class>` for the token classes of the engine.
- The engine is asked after every change. An answer for a text that has changed since is
  dropped.
