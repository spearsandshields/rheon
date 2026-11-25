# @rheon/feel-codemirror

CodeMirror 6 support for FEEL from the rheon engine's own parser. The colours, the error marks,
the completions and the hint of the invoked function are what the engine answers for the text, so the editor reads a text as
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
| `check` | Check a text that parses too: an unknown name or entry, a deprecated name, a value that is none of those listed, an operation that is always null. Marked as warnings and hints | false |
| `onSyntax` | Called with every answer the editor takes | none |
| `signatureHelp` | Inside the parentheses of a built-in function or a function of the outline, show its signatures with the parameter of the cursor marked | true |
| `hover` | When the pointer rests on a name, show its type, or the signatures of a function, and what the outline or the engine says of it. Needs an engine that answers `hover` (a `FeelEngine` and `instanceSource` do) | true |

- `syntaxOf(state)` is the last answer the editor took (tokens, errors, nodes).
- Colours are CSS custom properties with defaults: `--feel-keyword`, `--feel-number`,
  `--feel-string`, `--feel-constant`, `--feel-function`, `--feel-property`, `--feel-name`,
  `--feel-comment`, `--feel-operator`, `--feel-punctuation`, `--feel-error`. The classes are
  `cm-feel-<class>` for the token classes of the engine.
- The engine is asked after every change. An answer for a text that has changed since is
  dropped.
- A built-in function is completed with its parentheses and shows its signature and what it
  gives. A parameter that can be named is completed with its colon. The hint has the classes
  `cm-feel-signature`, `-line`, `-active`, `-given` and `-summary`; `signatureHint(call)` builds
  its element from the `call` of a `complete` answer.
- What shows over a name has the classes `cm-feel-hover`, `-line`, `-documentation` and
  `-deprecated`; `hoverInfo(hover, name)` builds its element from the `hover` of a `hover`
  answer, and `hoverAt(engine, view, pos)` gives the tooltip for a place.
- Tests: `npm test` runs the extension in jsdom with the engine in the same thread.
