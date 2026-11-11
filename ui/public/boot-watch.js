// The boot screen's way out (the 0.4.1 persona walks: a start that failed
// left "Opening AI Juggler…" up for good, with nothing to read or press).
// If the app hasn't replaced the boot screen after 20 s, or its code fails
// to load first, the screen says so and offers Reload. Its own file: the
// page's policy allows no inline script.
(function () {
  var shown = false;
  function stuck(why) {
    var boot = document.getElementById("boot");
    if (!boot || shown) return;
    shown = true;
    boot.setAttribute("role", "alert");
    var line = document.createElement("p");
    line.textContent = why;
    line.style.margin = "0";
    var again = document.createElement("button");
    again.type = "button";
    again.textContent = "Reload";
    again.style.cssText = "font:inherit;color:inherit;background:transparent;border:1px solid currentColor;border-radius:6px;padding:4px 12px;cursor:pointer";
    again.addEventListener("click", function () {
      location.reload();
    });
    var text = boot.lastChild;
    if (text && text.nodeType === 3) boot.removeChild(text);
    boot.appendChild(line);
    boot.appendChild(again);
  }
  window.addEventListener("error", function () {
    stuck("AI Juggler couldn't finish opening. Reload to try again; if it keeps happening, quit and open it again.");
  });
  setTimeout(function () {
    stuck("AI Juggler is taking longer than usual to open. Reload to try again.");
  }, 20000);
})();
