// The theme the user chose, on <html data-theme> before the first paint
// (the same rule as src/theme.ts: stored choice, else dark; "system"
// follows macOS). Its own file: the page's policy allows no inline script.
(function () {
  var choice = "dark";
  try {
    var stored = localStorage.getItem("aij.theme");
    if (stored === "light" || stored === "dark" || stored === "system") choice = stored;
  } catch (e) {}
  if (choice === "system") {
    choice = window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  }
  document.documentElement.dataset.theme = choice;
})();
