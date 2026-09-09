(function () {
  try {
    var raw = localStorage.getItem("openmesh.appearance");
    var a = raw ? JSON.parse(raw) : null;
    var theme = (a && a.theme) || "dark";
    var fontSize = (a && a.fontSize) || "medium";
    var density = (a && a.density) || "comfortable";
    var dark =
      theme === "dark" ||
      (theme === "system" &&
        window.matchMedia("(prefers-color-scheme: dark)").matches);
    var root = document.documentElement;
    root.classList.toggle("dark", !!dark);
    root.dataset.theme = dark ? "dark" : "light";
    root.dataset.fontSize = fontSize;
    root.dataset.density = density;
    root.style.colorScheme = dark ? "dark" : "light";
  } catch {
    document.documentElement.classList.add("dark");
  }
})();
