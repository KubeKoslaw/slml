// Only top-level GET navigations are intercepted. Fetching sources from the
// viewer is an XHR request and therefore cannot redirect itself recursively.
export function redirect(details, viewer) {
  const url = new URL(details.url);
  if (details.method !== "GET" || details.type !== "main_frame" ||
      !["http:", "https:"].includes(url.protocol) || !url.pathname.endsWith(".slml")) return {};
  return { redirectUrl: `${viewer}?url=${encodeURIComponent(url.href)}` };
}

if (typeof browser !== "undefined") {
  const viewer = browser.runtime.getURL("viewer.html");
  browser.webRequest.onBeforeRequest.addListener(
    details => redirect(details, viewer),
    { urls:["http://*/*", "https://*/*"], types:["main_frame"] },
    ["blocking"]
  );
  browser.action.onClicked.addListener(() => browser.tabs.create({ url:viewer }));
}
