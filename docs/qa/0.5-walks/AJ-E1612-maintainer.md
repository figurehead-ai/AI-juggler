# The 0.5 sign-in sitting (second time): your one page

Agents drive everything else. They set up the lab, press each Sign in in AI Juggler Dev, answer each tool's menu in AI Juggler's terminal, and run every check. The whole run has already been rehearsed on stand-in tools. Your part is one browser page per tool, one page at a time. No step needs a password typed into AI Juggler, and no app window of the tools opens.

Each tool has two lab Apps in AI Juggler Dev. You sign in **lab A** only. **Lab B** stays signed out, which proves that your sign-in stays in lab A. Your real setups aren't touched; the agents check that afterwards.

The agents tell you when a page is waiting. Each page waits up to 15 minutes. If it runs out, they stop that sign-in and move on, and nothing is lost. All the sign-ins come one after another. After the last one you're done: the agents run the rest without you, and the lab stays until every check has a result.

| # | Tool | What appears | Your one step |
|---|---|---|---|
| 1 | ChatGPT, for the 0.4.6 line's proof | An OpenAI page in your browser, opened by `codex login` in a lab folder. No ChatGPT app opens, and your own `~/.codex` isn't used. | Sign in with your ChatGPT account and allow Codex. The page then says you can close it. |
| 2 | Grok Build, for 0.4.6 | An xAI sign-in page from the 0.4.6 session's own lab. | Sign in with your xAI or X account. |
| 3 | OpenCode | An OpenAI page. The agent has already chosen OpenAI, then "ChatGPT Pro/Plus (browser)", in OpenCode's menu, so no API key is asked for. | Sign in with ChatGPT, using your existing ChatGPT subscription, and allow OpenCode. |
| 4 | Kimi Code | A Kimi (Moonshot) page with a device code. AI Juggler's terminal shows the same code. | Sign in to your Kimi account, check the code matches, approve. |
| 5 | Grok Build | An xAI sign-in page from `grok login`. | Sign in with your xAI or X account. (This is the second Grok sign-in. Each line has its own lab.) |
| 6 | ZCode | A Z.AI sign-in page. No ZCode window opens. | Sign in with the same Z.AI account as your ZCode. Your own ZCode stays signed in as it is. |
| 7 | Muse Code | A Meta page asking you to approve a sign-in code (`muse login`, kept in the lab's own file, not your Keychain). | Approve the code with your Meta account. **Only if the agent reports that your real Muse sign-in's date changed:** run `muse login` once in your own terminal to restore it. |
| 8 | Hermes, only if you've installed it | Hermes asks which provider to use. The agent asks you which one, then a page from that provider opens. | Name the provider, then sign in on its page. Not installed yet: this row is skipped, and nothing is installed for you. |

Rows 1, 2 and the 0.4.6 session's OpenCode check afterwards are driven by the 0.4.6 session in the same sitting. It uses lab A of OpenCode as it is: there's no second OpenCode sign-in.

ChatGPT (row 1) and OpenCode (row 3) both use the same local port for their pages, so they never run at the same time.

If a page asks for anything other than signing in, approving a code, or allowing the tool, close it and tell the agent.

Afterwards the agents record, for each tool: separate sign-ins, a session started and resumed, usage read, and lab B kept out. The coordinating session checks the evidence. Only then is the lab removed, and your lab sign-ins with it.
