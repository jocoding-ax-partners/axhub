package sample;

class Sample {
    void load(Client client, String ownerId) {
        var posts = client.table("posts").eq("owner_id", ownerId).limit(20).list();
        var total = client.table("posts").count();
        System.out.println(posts + total);
    }
}
