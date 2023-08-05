const Column = {
    FRESH: "Fresh",
    SAVED: "Saved",
    ARCHIVED: "Archived",
};

let currentColumn = Column.FRESH;
let currentArticle = {
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

        columns[Column.FRESH] = document.getElementById("articles-center");
        columns[Column.SAVED] = document.getElementById("articles-left");
        columns[Column.ARCHIVED] = document.getElementById("articles-right");

        // Sort articles by recent first
        data.sort(
            (a, b) =>
                new Date(b.published.replace(" ", "T").replace(" ", "")) -
                new Date(a.published.replace(" ", "T").replace(" ", ""))
        );

        for (const article of data) {
            const articleElement = document.createElement("div");
            const published = format_time_ago(article.published);

            articleElement.innerHTML = `
                <a class="article-link" href="${article.link}">${article.title}</a>
                <div class="article-date">${published}</div>
                <img class="article-icon" src="${article.channel.icon}">
            `;
            articleElement.classList.add("article");

            if (article.read_status === Column.FRESH) {
                columns[Column.FRESH].appendChild(articleElement);
            } else if (article.read_status === Column.SAVED) {
                columns[Column.SAVED].appendChild(articleElement);
            } else if (article.read_status === Column.ARCHIVED) {
                columns[Column.ARCHIVED].appendChild(articleElement);
            }
        }

        highlightCurrentArticle();
    } catch (error) {
        console.error(error);
    }
};

const highlightCurrentArticle = () => {
    // Remove the 'selected' and 'first' class from all articles
    document.querySelectorAll(".article").forEach((el) => {
        el.classList.remove("selected");
        el.classList.remove("first");
    });

    // Add the 'selected' class to the current article in the current column
    const articles = columns[currentColumn].getElementsByClassName("article");
    if (articles.length > currentArticle[currentColumn]) {
        articles[currentArticle[currentColumn]].classList.add("selected");
    }

    // Reorder the articles in the current column
    const selectedArticleIndex = currentArticle[currentColumn] - 1;
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

    // // Add first class to first article to give it margin
    // if (articles.length > 1) {
    //     if (currentArticle[currentColumn] === 1) {
    //         articles[0].classList.remove("first");
    //     } else {
    //         articles[0].classList.add("first");
    //     }
    // }
};

document.addEventListener("keydown", async (event) => {
    const articles = columns[currentColumn].getElementsByClassName("article");
    let articleToMove;

    switch (event.key) {
        case "q":
            // Change current column to the previous one
            currentColumn =
                currentColumn === Column.FRESH
                    ? Column.SAVED
                    : currentColumn === Column.SAVED
                    ? Column.ARCHIVED
                    : Column.FRESH;
            break;
        case "e":
            // Change current column to the next one
            currentColumn =
                currentColumn === Column.FRESH
                    ? Column.ARCHIVED
                    : currentColumn === Column.SAVED
                    ? Column.FRESH
                    : Column.SAVED;
            break;
        case "w":
            // Change current article to the previous one in the current column, cyclical
            currentArticle[currentColumn] =
                (currentArticle[currentColumn] - 1 + articles.length) %
                articles.length;
            break;
        case "s":
            // Change current article to the next one in the current column, cyclical
            currentArticle[currentColumn] =
                (currentArticle[currentColumn] + 1) % articles.length;
            break;
        case "a":
            // If current column has articles
            if (columns[currentColumn].childElementCount !== 0) {
                // Move the current article to the previous column in a cyclical manner
                articleToMove = articles[currentArticle[currentColumn]];
                if (currentColumn === Column.FRESH) {
                    columns[Column.SAVED].appendChild(articleToMove);
                } else if (currentColumn === Column.SAVED) {
                    columns[Column.ARCHIVED].appendChild(articleToMove);
                } else if (currentColumn === Column.ARCHIVED) {
                    columns[Column.FRESH].appendChild(articleToMove);
                }
            }
            break;
        case "d":
            // If current column has articles
            if (columns[currentColumn].childElementCount !== 0) {
                // Move the current article to the next column in a cyclical manner
                articleToMove = articles[currentArticle[currentColumn]];
                if (currentColumn === Column.FRESH) {
                    columns[Column.ARCHIVED].appendChild(articleToMove);
                } else if (currentColumn === Column.SAVED) {
                    columns[Column.FRESH].appendChild(articleToMove);
                } else if (currentColumn === Column.ARCHIVED) {
                    columns[Column.SAVED].appendChild(articleToMove);
                }
            }
            break;
    }

    // Clamp the current article to the bounds of the current column
    if (currentArticle[currentColumn] > articles.length - 1) {
        currentArticle[currentColumn] = 0;
    }

    // Highlight the current article
    highlightCurrentArticle();

    // // Send a PUT request to the server to update the article's status
    // if (articleToMove) {
    //     await fetch(`/articles/${articleToMove.id}`, {
    //         method: "PUT",
    //         headers: {
    //             "Content-Type": "application/json",
    //         },
    //         body: JSON.stringify({ read_status: currentColumn }),
    //     });
    // }
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
    published = published.replace(" ", "T").replace(" ", "");
    let publishedDate = new Date(published);

    // Calculate the duration between the current time and the published date
    let duration = Math.floor(
        (new Date().getTime() - publishedDate.getTime()) / 1000
    );

    // Depending on the duration, format it in different ways
    if (duration < 60) {
        return duration + "s";
    } else {
        let mins = duration / 60;
        if (mins < 10) {
            return formatDecimal(mins) + "m";
        } else if (mins < 60) {
            return Math.floor(mins) + "m";
        } else {
            let hours = mins / 60;
            if (hours < 6) {
                return formatDecimal(hours) + "h";
            } else if (hours < 24) {
                return Math.floor(hours) + "h";
            } else {
                let days = hours / 24;
                if (days < 7) {
                    return formatDecimal(days) + "d";
                } else {
                    let weeks = days / 7;
                    if (weeks < 4) {
                        return formatDecimal(weeks) + "w";
                    } else {
                        let months = days / 30;
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
    let formattedValue = value.toFixed(1);
    return formattedValue.endsWith(".0")
        ? formattedValue.slice(0, -2)
        : formattedValue;
}
