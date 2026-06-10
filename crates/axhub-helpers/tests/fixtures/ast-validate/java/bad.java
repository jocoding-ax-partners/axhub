package sample;

// 잘못된 SDK 사용 (block 룰 검출 대상).
class Sample {
    void loadPosts(Client client) {
        var rows = client.table("posts").where(Ops.or(eq("a", 1), eq("b", 2))).list();
        var n = client.table("posts").where(Ops.not(eq("a", 1))).count();
        var page = client.table("posts").after("cursor").list();
        System.out.println(rows + n + page);
    }
}
