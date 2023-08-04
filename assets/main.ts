


/// Format the published date to a human readable format, -30s, -2m 5s, -1h 30m, etc
function format_time_ago(published: string): string {
    // Parse the published date
    let publishedDate: Date = new Date(published);

    // Calculate the duration between the current time and the published date
    let duration: number = Math.floor((new Date().getTime() - publishedDate.getTime()) / 1000);

    // Depending on the duration, format it in different ways
    if (duration < 60) {
        return "-" + duration + "s";
    } else {
        let mins: number = Math.floor(duration / 60);
        if (mins < 60) {
            return "-" + mins + "m " + duration % 60 + "s";
        } else {
            let hours: number = Math.floor(mins / 60);
            if (hours < 24) {
                return "-" + hours + "h " + mins % 60 + "m";
            } else {
                let days: number = Math.floor(hours / 24);
                if (days < 7) {
                    return "-" + days + "d " + hours % 24 + "h";
                } else {
                    let weeks: number = Math.floor(days / 7);
                    if (weeks < 4) {
                        return "-" + weeks + "w " + days % 7 + "d";
                    } else {
                        return "-" + Math.floor(days / 30) + "m " + days % 30 + "d";
                    }
                }
            }
        }
    }
}
