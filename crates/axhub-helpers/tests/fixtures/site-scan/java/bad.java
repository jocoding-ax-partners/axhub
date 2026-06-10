package sample;

import java.net.http.HttpClient;
import java.sql.DriverManager;

class Sample {
    void load() throws Exception {
        var http = HttpClient.newHttpClient();
        var conn = DriverManager.getConnection("jdbc:postgresql://db/x");
        String url = "https://api.axhub.dev/v1/posts";
        System.out.println(http + conn + url);
    }
}
