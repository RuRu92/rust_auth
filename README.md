OVERVIEW
==================



TODO
==================




LINUX COMMANDS 
===================
Copy all content from dir to another
$ cp -r /path/to/source/* /path/to/destination/  

Delete all content of a dir
$ rm -rf foo/app/*



DOCKER COMMANDS
==================
docker ps


docker stop <container_name_or_id>
docker rm <container_name_or_id>

docker rmi <image_name_or_id>

docker exec -it <mysql_container_name> bash
mysql -u <username> -p


Add new column `last_login` to `realm_user` table (MySQL):

```sql
ALTER TABLE realm_user
ADD COLUMN last_login DATETIME NULL AFTER auth_token;
```

To update the `last_login` value for a user:

```sql
UPDATE realm_user
SET last_login = NOW()
WHERE user_id = '<user_id>';
```

