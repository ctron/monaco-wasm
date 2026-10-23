// Renders examples.json: pinned examples as cards, the others in the list below.
// Examples without a build are not hosted and link to their source instead.
// Examples open in a new tab, so the overview stays open.

const repository = document.querySelector(".links a").href.replace(/\/$/, "");
const { examples } = await (await fetch("examples.json")).json();

function element(tag, attributes = {}, ...children) {
  const node = Object.assign(document.createElement(tag), attributes);
  node.append(...children);
  return node;
}

// Backticks in descriptions mark code.
function text(markup) {
  return markup.split("`").map((part, index) => (index % 2 ? element("code", {}, part) : part));
}

const newTab = { target: "_blank", rel: "noopener" };
const sourceUrl = (example) => `${repository}/tree/main/examples/${example.source}`;

document.querySelector(".cards").append(
  ...examples
    .filter((example) => example.pinned)
    .map((example) =>
      element(
        "article",
        { className: "card" },
        element("h2", {}, example.title),
        element("p", {}, ...text(example.description)),
        ...example.snippets.map((snippet) => element("pre", {}, element("code", {}, snippet))),
        element("a", { className: "button", href: `${example.id}/`, ...newTab }, example.link),
      ),
    ),
);

document.querySelector(".example-list").append(
  ...examples
    .filter((example) => !example.pinned)
    .map((example) =>
      element(
        "li",
        {},
        element("h3", {}, example.title),
        element("p", {}, ...text(example.description)),
        element(
          "p",
          { className: "example-links" },
          ...(example.build ? [element("a", { className: "open", href: `${example.id}/`, ...newTab }, "Open"), " · "] : []),
          element("a", { href: sourceUrl(example) }, "Source"),
        ),
      ),
    ),
);
