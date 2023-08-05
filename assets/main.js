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
    [Column.FRESH]: null,
    [Column.SAVED]: null,
    [Column.ARCHIVED]: null,
};

const fetchArticles = async () => {
    try {
        const response = await fetch("/articles");
        const data = await response.json();
        for (const article of data) {
            article.published = new Date(
                article.published.replace(" ", "T").replace(" ", "")
            );
        }

        columns[Column.FRESH] = document.getElementById("articles-center");
        columns[Column.SAVED] = document.getElementById("articles-left");
        columns[Column.ARCHIVED] = document.getElementById("articles-right");

        // Sort articles by recent first
        data.sort((a, b) => b.published - a.published);

        for (const article of data) {
            const articleElement = document.createElement("div");
            const published = format_time_ago(article.published);

            articleElement.innerHTML = `
                <a class="article-link" href="${article.link}">${article.title}</a>
                <div class="article-date">${published}</div>
                <img class="article-icon" src="${article.channel.icon}">
            `;
            articleElement.classList.add("article");
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
                const articles =
                    columns[column].getElementsByClassName("article");
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
    // Remove the 'selected' and 'first' class from all articles and boxes
    document.querySelectorAll(".article").forEach((el) => {
        el.classList.remove("selected");
        el.classList.remove("first");
    });
    document.querySelectorAll(".articlebox-column").forEach((el) => {
        el.classList.remove("selected");
        el.parentElement.classList.remove("selected");
    });

    // Add the 'selected' class to the current article in the current column and to the column
    const articles = columns[currentColumn].getElementsByClassName("article");
    if (articles.length > currentArticle[currentColumn]) {
        articles[currentArticle[currentColumn]].classList.add("selected");
    }
    columns[currentColumn].classList.add("selected");
    columns[currentColumn].parentElement.classList.add("selected");

    // Reorder the articles in the current column
    const selectedArticleIndex = currentArticle[currentColumn];
    for (let i = 0; i < articles.length; i++) {
        let newIndex = i - selectedArticleIndex;
        if (i < selectedArticleIndex) {
            newIndex = articles.length - selectedArticleIndex + i;
        }
        if (newIndex >= articles.length) {
            newIndex = i - articles.length;
        }
        articles[i].style.order = newIndex;
    }
};

const undoStack = [];
const redoStack = [];

const moveArticle = (article, fromColumnStatus, toColumnStatus) => {
    const fromColumn = columns[fromColumnStatus];
    const toColumn = columns[toColumnStatus];

    toColumn.appendChild(article);
    undoStack.push({ article, fromColumn, toColumn });
    redoStack.length = 0; // Clear the redo stack whenever a new move is made
    if (undoStack.length > 10) {
        undoStack.shift();
    }

    // Keep the current article in bounds
    if (
        currentArticle[currentColumn] >=
        columns[currentColumn].childElementCount
    ) {
        currentArticle[currentColumn] = 0;
    }

    // Sort the articles in the destination column by date
    Array.from(toColumn.children)
        .sort((a, b) => b.data.published - a.data.published)
        .forEach((articleElement) => toColumn.appendChild(articleElement));

    // Send a PUT request to the server to update the article's read status
    const articleLink = encodeURIComponent(article.data.link);
    fetch(`/articles/${articleLink}/${toColumnStatus}`, { method: "PUT" })
        .then((response) => response.json())
        .catch((error) => console.error("Error moving article:", error));
};

document.addEventListener("keydown", async (event) => {
    const articles = columns[currentColumn].getElementsByClassName("article");
    let articleToMove;

    switch (event.key) {
        // Select article to the left
        case "q":
            // Change current column to the previous one
            currentColumn =
                currentColumn === Column.FRESH
                    ? Column.SAVED
                    : currentColumn === Column.SAVED
                    ? Column.ARCHIVED
                    : Column.FRESH;
            localStorage.setItem("currentColumn", currentColumn);
            break;
        // Select column to the right
        case "e":
            // Change current column to the next one
            currentColumn =
                currentColumn === Column.FRESH
                    ? Column.ARCHIVED
                    : currentColumn === Column.SAVED
                    ? Column.FRESH
                    : Column.SAVED;
            localStorage.setItem("currentColumn", currentColumn);
            break;
        // Select article above
        case "w":
            // Change current article to the previous one in the current column, cyclical
            currentArticle[currentColumn] =
                (currentArticle[currentColumn] - 1 + articles.length) %
                articles.length;
            localStorage.setItem(
                "currentArticle",
                JSON.stringify(currentArticle)
            );
            break;
        // Select article below
        case "s":
            // Change current article to the next one in the current column, cyclical
            currentArticle[currentColumn] =
                (currentArticle[currentColumn] + 1) % articles.length;
            localStorage.setItem(
                "currentArticle",
                JSON.stringify(currentArticle)
            );
            break;
        // Move article left
        case "a":
            if (columns[currentColumn].childElementCount !== 0) {
                articleToMove = articles[currentArticle[currentColumn]];
                const toColumn =
                    currentColumn === Column.FRESH
                        ? Column.SAVED
                        : currentColumn === Column.SAVED
                        ? Column.ARCHIVED
                        : Column.FRESH;
                moveArticle(articleToMove, currentColumn, toColumn);
            }
            break;
        // Move article right
        case "d":
            if (columns[currentColumn].childElementCount !== 0) {
                articleToMove = articles[currentArticle[currentColumn]];
                const toColumn =
                    currentColumn === Column.FRESH
                        ? Column.ARCHIVED
                        : currentColumn === Column.SAVED
                        ? Column.FRESH
                        : Column.SAVED;
                moveArticle(articleToMove, currentColumn, toColumn);
            }
            break;
        // Undo
        case "z":
            if (event.ctrlKey && undoStack.length > 0) {
                const { article, fromColumn, toColumn } = undoStack.pop();
                redoStack.push({
                    article,
                    fromColumn: toColumn,
                    toColumn: fromColumn,
                });
                fromColumn.appendChild(article);
            }
            break;
        // Redo
        case "Z":
            if (event.ctrlKey && event.shiftKey && redoStack.length > 0) {
                const { article, fromColumn, toColumn } = redoStack.pop();
                undoStack.push({
                    article,
                    fromColumn: toColumn,
                    toColumn: fromColumn,
                });
                fromColumn.appendChild(article);
            }
            break;
        // Reset to first article in current column
        case "r":
            currentArticle[currentColumn] = 0;
            localStorage.setItem(
                "currentArticle",
                JSON.stringify(currentArticle)
            );
            break;
    }

    highlightCurrentArticle();
});

fetchArticles();

// This function takes a date string as input and returns a string representing how much time has passed since that date.
// The output format is as follows:
// - Seconds if under a minute (e.g., "-5s")
// - Minutes with 1 decimal place if under 10 minutes (e.g., "-4.5m")
// - Just minutes if under 1 hour (e.g., "-20m")
// - Hours with 1 decimal place if under 6 hours (e.g., "-1.5h")
// - Just hours if under 24 hours
// - Days with 1 decimal place if under a week
// - Weeks with 1 decimal place if under a month
// - Months with 1 decimal place if under a year
// - Year with 1 decimal place after that
function format_time_ago(published) {
    // Calculate the duration between the current time and the published date
    const duration = Math.floor(
        (new Date().getTime() - published.getTime()) / 1000
    );

    // Depending on the duration, format it in different ways
    if (duration < 60) {
        return duration + "s";
    } else {
        const mins = duration / 60;
        if (mins < 10) {
            return formatDecimal(mins) + "m";
        } else if (mins < 60) {
            return Math.floor(mins) + "m";
        } else {
            const hours = mins / 60;
            if (hours < 6) {
                return formatDecimal(hours) + "h";
            } else if (hours < 24) {
                return Math.floor(hours) + "h";
            } else {
                const days = hours / 24;
                if (days < 7) {
                    return formatDecimal(days) + "d";
                } else {
                    const weeks = days / 7;
                    if (weeks < 4) {
                        return formatDecimal(weeks) + "w";
                    } else {
                        const months = days / 30;
                        if (months < 12) {
                            return formatDecimal(months) + "m";
                        } else {
                            return formatDecimal(days / 365) + "y";
                        }
                    }
                }
            }
        }
    }
}

function formatDecimal(value) {
    const formattedValue = value.toFixed(1);
    return formattedValue.endsWith(".0")
        ? formattedValue.slice(0, -2)
        : formattedValue;
}
