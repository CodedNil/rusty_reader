const Column = {
    FRESH: "Fresh",
    SAVED: "Saved",
    ARCHIVED: "Archived",
};

let currentColumn = localStorage.getItem("currentColumn") || Column.FRESH;
let currentArticle = JSON.parse(localStorage.getItem("currentArticle")) || {
    [Column.FRESH]: 0,
    [Column.SAVED]: 0,
    [Column.ARCHIVED]: 0,
};

const columns = {
    [Column.FRESH]: document.getElementById("articles-center"),
    [Column.SAVED]: document.getElementById("articles-left"),
    [Column.ARCHIVED]: document.getElementById("articles-right"),
};

const fetchArticles = async () => {
    try {
        const response = await fetch("/articles");
        const data = await response.json();
        for (const article of data) {
            article.published = new Date(article.published);
        }

        // Sort articles by recent first
        data.sort((a, b) => b.published - a.published);

        for (const article of data) {
            const articleElement = document.createElement("div");
            const published = format_time_ago(article.published);

            articleElement.innerHTML = `
                <a class="article-link" href="${article.link}">${article.title}</a>
                <div class="article-details">
                    <img class="article-icon" src="${article.channel.icon}">
                    <div class="article-date">${published}</div>
                </div>
            `;
            articleElement.classList.add("article");

            // Set the background color of the article to the dominant color of the channel
            let dominantColor = article.channel.dominant_color;
            let color = tinycolor(dominantColor).toHsl();
            color.s = 0.5;
            color.l = 0.4;
            let color_selected = tinycolor(dominantColor).toHsl();
            color_selected.s = 0.6;
            color_selected.l = 0.6;
            if (article.read_status === Column.ARCHIVED) {
                color.s = 0.3;
                color_selected.s = 0.4;
            }
            articleElement.style.backgroundColor = tinycolor(color).toString();

            article.color = tinycolor(color).toString();
            article.color_selected = tinycolor(color_selected).toString();

            articleElement.data = article;

            if (article.read_status === Column.FRESH) {
                columns[Column.FRESH].appendChild(articleElement);
            } else if (article.read_status === Column.SAVED) {
                columns[Column.SAVED].appendChild(articleElement);
            } else if (article.read_status === Column.ARCHIVED) {
                columns[Column.ARCHIVED].appendChild(articleElement);
            }
        }

        // Ensure currentArticle is within bounds for each column
        for (const column in currentArticle) {
            if (currentArticle.hasOwnProperty(column)) {
                const articles = columns[column].getElementsByClassName("article");
                if (currentArticle[column] >= articles.length) {
                    currentArticle[column] = 0;
                }
            }
        }
        // Save the possibly updated currentArticle back to localStorage
        localStorage.setItem("currentArticle", JSON.stringify(currentArticle));

        highlightCurrentArticle();
    } catch (error) {
        console.error(error);
    }
};

const highlightCurrentArticle = () => {
    // Remove the 'selected' class from all articles and boxes
    document.querySelectorAll(".article").forEach((el) => {
        el.classList.remove("selected");
        el.style.backgroundColor = el.data.color;
    });
    document.querySelectorAll(".articlebox").forEach((el) => {
        el.classList.remove("selected");
        el.parentElement.classList.remove("selected");
    });

    // Add the 'selected' class to the current article in the current column and to the column
    const articles = columns[currentColumn].getElementsByClassName("article");
    if (articles.length > currentArticle[currentColumn]) {
        let article = articles[currentArticle[currentColumn]];
        article.classList.add("selected");
        article.style.backgroundColor = article.data.color_selected;
        article.scrollIntoView({ behavior: "smooth", block: "center" });

        // Setup preview
        setupPreview(article);
    }
    columns[currentColumn].classList.add("selected");
    columns[currentColumn].parentElement.classList.add("selected");
};

const setupPreview = (articleElement) => {
    document.getElementById("preview-header").innerHTML = articleElement.data.title;
    document.getElementById("preview-date").innerHTML = articleElement.data.published.toDateString();
    document.getElementById("preview-text").innerHTML = articleElement.data.summary;
    document.getElementById("preview-image").src = articleElement.data.image;
};

const undoStack = [];
const redoStack = [];
const columnsMap = {
    Fresh: { left: "Saved", right: "Archived" },
    Saved: { left: "Archived", right: "Fresh" },
    Archived: { left: "Fresh", right: "Saved" },
};

const moveArticle = (article, fromColumnStatus, toColumnStatus) => {
    const fromColumn = columns[fromColumnStatus];
    const toColumn = columns[toColumnStatus];

    toColumn.appendChild(article);

    // Push to undo stack
    undoStack.push({ article, fromColumn, toColumn });
    redoStack.length = 0; // Clear the redo stack whenever a new move is made
    if (undoStack.length > 10) undoStack.shift();

    sortArticlesByDate(toColumn);

    // Send a PUT request to the server to update the article's read status
    fetch(`/articles/${encodeURIComponent(article.data.link)}/${toColumnStatus}`, { method: "PUT" })
        .then((response) => response.json())
        .catch((error) => console.error("Error moving article:", error));

    highlightCurrentArticle();
};

const sortArticlesByDate = (column) => {
    Array.from(column.children)
        .sort((a, b) => b.data.published - a.data.published)
        .forEach((articleElement) => column.appendChild(articleElement));
};

document.addEventListener("keydown", async (event) => {
    const articles = columns[currentColumn].getElementsByClassName("article");
    let changesMade = false;

    switch (event.key) {
        case "q":
        case "e":
            currentColumn = columnsMap[currentColumn][event.key === "q" ? "left" : "right"];
            localStorage.setItem("currentColumn", currentColumn);
            highlightCurrentArticle();
            break;
        case "w":
        case "s":
            currentArticle[currentColumn] =
                (currentArticle[currentColumn] + (event.key === "w" ? -1 : 1) + articles.length) % articles.length;
            localStorage.setItem("currentArticle", JSON.stringify(currentArticle));
            highlightCurrentArticle();
            break;
        case "a":
        case "d":
            if (columns[currentColumn].childElementCount !== 0) {
                const articleToMove = articles[currentArticle[currentColumn]];
                const toColumn = columnsMap[currentColumn][event.key === "a" ? "left" : "right"];
                moveArticle(articleToMove, currentColumn, toColumn);
            }
            break;
        case "Enter":
            if (columns[currentColumn].childElementCount !== 0)
                window.open(articles[currentArticle[currentColumn]].data.link);
            break;
        case "z":
            if (event.ctrlKey && undoStack.length > 0) {
                const { article, fromColumn, toColumn } = undoStack.pop();
                redoStack.push({ article, fromColumn: toColumn, toColumn: fromColumn });
                fromColumn.appendChild(article);
                sortArticlesByDate(fromColumn);
                highlightCurrentArticle();
            }
            break;
        case "Z":
            if (event.ctrlKey && event.shiftKey && redoStack.length > 0) {
                const { article, fromColumn, toColumn } = redoStack.pop();
                undoStack.push({ article, fromColumn: toColumn, toColumn: fromColumn });
                fromColumn.appendChild(article);
                sortArticlesByDate(fromColumn);
                highlightCurrentArticle();
            }
            break;
        case "r":
            currentArticle[currentColumn] = 0;
            localStorage.setItem("currentArticle", JSON.stringify(currentArticle));
            highlightCurrentArticle();
            break;
    }
});

fetchArticles();

// Formats the time difference between the current time and the provided date.
function format_time_ago(published) {
    // Calculate the duration in seconds between the current time and the published date.
    const duration = (new Date().getTime() - published.getTime()) / 1000;

    // Define units and their respective limits, divisors, labels, and whether they should be formatted with a decimal.
    const units = [
        { limit: 60, divisor: 1, label: "s" },
        { limit: 600, divisor: 60, label: "m", decimal: true },
        { limit: 3600, divisor: 60, label: "m" },
        { limit: 21600, divisor: 3600, label: "h", decimal: true },
        { limit: 86400, divisor: 3600, label: "h" },
        { limit: 604800, divisor: 86400, label: "d", decimal: true },
        { limit: 2419200, divisor: 604800, label: "w", decimal: true },
        { limit: 29030400, divisor: 2592000, label: "mo", decimal: true }, // Assumes 30 days in a month.
        { divisor: 31536000, label: "y", decimal: true },
    ];

    // Iterate through the units to determine the appropriate format.
    for (let unit of units) {
        if (duration < unit.limit || !unit.limit) {
            const value = duration / unit.divisor;
            if (unit.decimal) {
                const formattedValue = value.toFixed(1);
                return (formattedValue.endsWith(".0") ? formattedValue.slice(0, -2) : formattedValue) + unit.label;
            } else {
                return Math.floor(value) + unit.label;
            }
        }
    }
}
