const Column = {
    FRESH: "Fresh",
    SAVED: "Saved",
    ARCHIVED: "Archived",
};

const columns = {
    [Column.FRESH]: document.getElementById("articles-center"),
    [Column.SAVED]: document.getElementById("articles-left"),
    [Column.ARCHIVED]: document.getElementById("articles-right"),
};

let currentColumn = localStorage.getItem("currentColumn") || Column.FRESH;
let currentArticle = JSON.parse(localStorage.getItem("currentArticle")) || {
    [Column.FRESH]: 0,
    [Column.SAVED]: 0,
    [Column.ARCHIVED]: 0,
};

const createArticleElement = (article) => {
    const articleElement = document.createElement("div");
    articleElement.classList.add("article");
    articleElement.innerHTML = `
        <a class="article-link" href="${article.link}">${article.title}</a>
        <div class="article-details">
            <img class="article-icon" src="${article.channel.icon}">
            <div class="article-date">${format_time_ago(article.published)}</div>
        </div>
    `;

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

    return articleElement;
};

const fetchArticles = async () => {
    try {
        const response = await fetch("/articles");
        const data = await response.json();

        // Convert and sort articles
        const articles = data
            .map((article) => {
                article.published = new Date(article.published);
                return article;
            })
            .sort((a, b) => b.published - a.published);

        const articleElements = articles.map(createArticleElement);

        for (let i = 0; i < articles.length; i++) {
            columns[articles[i].read_status].appendChild(articleElements[i]);
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

    // Find the article by its link in the current column
    const articles = Array.from(columns[currentColumn].getElementsByClassName("article"));
    let selectedArticle = articles.find((article) => article.data.link === currentArticle[currentColumn]);

    // If the selected article isn't valid, select the first article in the current column
    if (!selectedArticle && articles.length > 0) {
        selectedArticle = articles[0];
        currentArticle[currentColumn] = selectedArticle.data.link;
        localStorage.setItem("currentArticle", JSON.stringify(currentArticle));
    }

    if (selectedArticle) {
        selectedArticle.classList.add("selected");
        selectedArticle.style.backgroundColor = selectedArticle.data.color_selected;
        selectedArticle.scrollIntoView({ behavior: "smooth", block: "center" });

        // Setup preview
        document.getElementById("preview-header").innerHTML = selectedArticle.data.title;
        document.getElementById("preview-date").innerHTML = selectedArticle.data.published.toDateString();
        document.getElementById("preview-text").innerHTML = selectedArticle.data.summary;
        document.getElementById("preview-image").src = selectedArticle.data.image;
    }
    columns[currentColumn].classList.add("selected");
    columns[currentColumn].parentElement.classList.add("selected");
};

const undoStack = [];
const redoStack = [];
const columnsMap = {
    [Column.FRESH]: { left: Column.SAVED, right: Column.ARCHIVED },
    [Column.SAVED]: { left: Column.ARCHIVED, right: Column.FRESH },
    [Column.ARCHIVED]: { left: Column.FRESH, right: Column.SAVED },
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
    const fragment = document.createDocumentFragment();
    Array.from(column.children)
        .sort((a, b) => b.data.published - a.data.published)
        .forEach((articleElement) => fragment.appendChild(articleElement));
    column.appendChild(fragment);
};

document.addEventListener("keydown", async (event) => {
    const articles = Array.from(columns[currentColumn].getElementsByClassName("article"));
    const currentIndex = articles.findIndex((article) => article.data.link === currentArticle[currentColumn]);

    switch (event.key) {
        case "q":
        case "e":
            currentColumn = columnsMap[currentColumn][event.key === "q" ? "left" : "right"];
            localStorage.setItem("currentColumn", currentColumn);
            highlightCurrentArticle();
            break;
        case "w":
        case "s":
            const newIndex = (currentIndex + (event.key === "w" ? -1 : 1) + articles.length) % articles.length;
            currentArticle[currentColumn] = articles[newIndex].data.link;
            localStorage.setItem("currentArticle", JSON.stringify(currentArticle));
            highlightCurrentArticle();
            break;
        case "a":
        case "d":
            if (columns[currentColumn].childElementCount !== 0) {
                const articleToMove = articles[currentIndex];
                const toColumn = columnsMap[currentColumn][event.key === "a" ? "left" : "right"];
                moveArticle(articleToMove, currentColumn, toColumn);

                // Update the currentArticle for the source column
                const nextArticle = articles[currentIndex + 1] || articles[currentIndex - 1];
                currentArticle[currentColumn] = nextArticle ? nextArticle.data.link : null;
                localStorage.setItem("currentArticle", JSON.stringify(currentArticle));
            }
            break;
        case "Enter":
            if (currentArticle[currentColumn]) {
                window.open(currentArticle[currentColumn]);
            }
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
            for (let col in columns) {
                const colArticles = Array.from(columns[col].getElementsByClassName("article"));
                const firstArticle = colArticles[0];
                if (firstArticle) {
                    currentArticle[col] = firstArticle.data.link;
                } else {
                    currentArticle[col] = null;
                }
            }
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
