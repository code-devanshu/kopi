export const THEME_STORAGE_KEY = "kopi-theme";

// Runs before first paint: apply the saved theme, otherwise follow the system.
const script = `(function(){try{var d=document.documentElement,s=localStorage.getItem("${THEME_STORAGE_KEY}"),m=window.matchMedia("(prefers-color-scheme: dark)");function a(){var t=localStorage.getItem("${THEME_STORAGE_KEY}");d.setAttribute("data-theme",t==="light"||t==="dark"?t:(m.matches?"dark":"light"))}a();m.addEventListener("change",a)}catch(e){}})()`;

export function ThemeScript() {
  return <script dangerouslySetInnerHTML={{ __html: script }} />;
}
