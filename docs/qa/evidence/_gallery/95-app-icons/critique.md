<!-- captures: dac7e29c8cd4 -->
# 95-app-icons: installed apps' own icons on Home, in the rail, in History and in the Go to session palette

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The app marks do their job at every size: Claude's and Terminal's own icons lead the Your apps cards (home-1280, home-2000), the rail's group headers (rail-1280, rail-2000) and the History rows (history-1280, history-2000), with the profile's colour dot beside the name rather than on the icon. ChatGPT has no icon in this fixture and falls back to a neutral lettered "C" tile, which sits quietly beside the real icons and does not look broken. On the 0.3.2 surfaces the app cards and the Needs you and Runway cards stand off the dark canvas and read as white cards on grey in light; Open its page and New session are the solid violet primaries, and the amber "1 needs you" chips and "Asks you" labels stay legible in both themes. As elsewhere on Home, the ChatGPT card, the only one without a "1 needs you" chip, drops Open ChatGPT to the chip's line, so it sits lower than the other cards' Open buttons (home-1280, home-2000). palette-1280-dark and palette-2000-dark were not re-rendered and still show the 0.3.1 colours; their layout (a centred Go to session list, an icon on every row, "open panel" / "open" on the right) is unchanged.

## Weak

- home-1280, home-2000: Open ChatGPT sits lower than Open Claude and Open Terminal in the same row of cards.
- palette-1280-dark, palette-2000-dark: not re-rendered this round.
- history-2000-dark: the list keeps a narrow centred column, so at 2000 wide the two rows sit in a band with most of the page empty; fine for two rows, thin for a real history.
