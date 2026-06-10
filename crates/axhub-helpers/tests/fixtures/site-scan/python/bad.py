import requests
import psycopg2


def load():
    res = requests.get("https://api.axhub.dev/v1/posts")
    conn = psycopg2.connect("dbname=x")
    url = "https://backend.example.com/data"
    return res, conn, url
