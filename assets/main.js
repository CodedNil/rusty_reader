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
                <a class="article-link" href="${article.link}">${article.title}</a>
                <div class="article-date">${published}</div>
                <img class="article-icon" src="${article.channel.icon}">
            `;
            // Add class article
            articleElement.classList.add("article");

            articlesCenter.appendChild(articleElement);
        }
    } catch (error) {
        console.error(error);
    }
};

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
