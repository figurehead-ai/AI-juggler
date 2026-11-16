# The 0.5 sign-in sitting: your one page

Agents drive everything else: they set up the lab, press each Sign in in AI Juggler Dev, and run every check. Your part is one step per tool, in your browser. No step needs a password typed into AI Juggler.

Each tool has two lab Apps in AI Juggler Dev. You sign in **lab A** only. **Lab B** stays signed out, to prove that your sign-in stays in lab A. Your real setups aren't touched. The agents check that afterwards.

The agents go one or two tools at a time and tell you when a page is waiting for you.

| Tool | What appears | Your one step |
|---|---|---|
| OpenCode | A browser page from OpenAI, opened by OpenCode's sign-in in lab A (the agent chose OpenAI → ChatGPT). | Sign in with ChatGPT, using your existing ChatGPT subscription. |
| Kimi Code | A Kimi (Moonshot) page asking for a device code. AI Juggler's terminal shows the same code. | Sign in to your Kimi account and approve the code. |
| Grok Build | An xAI sign-in page opened by `grok login`. | Sign in with your xAI or X account. |
| ZCode | A Z.AI sign-in page opened by ZCode's sign-in. No ZCode window opens. | Sign in with the same Z.AI account as your ZCode. Your own ZCode stays signed in as it is. |
| Muse Code | A Meta page asking you to approve a sign-in code (`muse login`, kept in the lab's own file, not your Keychain). | Approve the code with your Meta account. **Only if the agent reports that your real Muse sign-in's date changed:** run `muse login` once in your own terminal to restore it. |
| Grok Build (0.4.6) | A sign-in in the window AI Juggler Dev 0.4.6 opens. | Sign in with your xAI/X account. |
| ChatGPT app (Codex), for the 0.4.6 line's C2 proof | A separate ChatGPT window showing ChatGPT's sign-in. The 0.4.6 session's driver opens it with its own lab folders, so your real ChatGPT setup isn't used. | Sign in with your ChatGPT account. |

The ChatGPT app row is driven by the 0.4.6 session, in the same sitting. The 0.5 session hands it AI Juggler Dev and the screen for that step, and takes them back afterwards. Afterwards the driver checks that your real `~/.codex` is untouched (file dates by name only).

In the 0.4.6 session's turn it also uses T16's OpenCode lab A folder in place (no second OpenCode sign-in) and makes its own Grok home. That is why Grok signs in twice: once for 0.5's T16 lab A, once for 0.4.6.

If a page asks for anything other than signing in or approving a code, close it and tell the agent.

Afterwards the agents record what happened for each tool: separate sign-ins, a session started and resumed, and usage read. They then quit the lab and remove it.
