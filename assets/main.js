const fetchArticles = async () => {
    try {
        const response = await fetch("/articles");
        const data = await response.json();
        const articlesCenter = document.getElementById("articles-center");

        if (!articlesCenter) {
            throw new Error('Element with id "articles-center" not found');
        }

        for (const article of data) {
            const articleElement = document.createElement("div");
            const published = format_time_ago(article.published);

            articleElement.innerHTML = `
                <a class="feed-link" href="${article.link}">${article.title}</a>
                <div class="feed-date">${published}</div>
                <img class="feed-icon" src="${article.channel.icon}">
            `;
            articleElement.style.display = "flex";
            articleElement.style.flexDirection = "row";

            articlesCenter.appendChild(articleElement);
        }
    } catch (error) {
        console.error(error);
    }
};

fetchArticles();

/// Format the published date to a human readable format, -30s, -2m 5s, -1h 30m, etc
function format_time_ago(published) {
    published = published.replace(" ", "T").replace(" ", "");
    let publishedDate = new Date(published);

    // Calculate the duration between the current time and the published date
    let duration = Math.floor(
        (new Date().getTime() - publishedDate.getTime()) / 1000
    );

    // Depending on the duration, format it in different ways
    if (duration < 60) {
        return "-" + duration + "s";
    } else {
        let mins = Math.floor(duration / 60);
        if (mins < 60) {
            return "-" + mins + "m " + (duration % 60) + "s";
        } else {
            let hours = Math.floor(mins / 60);
            if (hours < 24) {
                return "-" + hours + "h " + (mins % 60) + "m";
            } else {
                let days = Math.floor(hours / 24);
                if (days < 7) {
                    return "-" + days + "d " + (hours % 24) + "h";
                } else {
                    let weeks = Math.floor(days / 7);
                    if (weeks < 4) {
                        return "-" + weeks + "w " + (days % 7) + "d";
                    } else {
                        return (
                            "-" +
                            Math.floor(days / 30) +
                            "m " +
                            (days % 30) +
                            "d"
                        );
                    }
                }
            }
        }
    }
}
