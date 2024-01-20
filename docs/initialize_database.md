## Initialization
To initialize the database in case of necessity (for example when you have a clean environment)
you should run the following commands from the terminal:
```
export DATABASE_URL="postgres://postgres:password@localhost:5431/mydb"
cargo sqlx migrate run
```

If you have done changes to some query, you'll have to run:
```
cargo sqlx prepare
```
it wil generate the offline queries to build the application