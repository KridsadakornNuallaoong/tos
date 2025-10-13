# GET IN TOS!!!

- What's TOS service?? 
--
**TOS: Tiny Orca Service** <<- It just a name ha ha.
- we make it for collect raw data in medium-high speed and we need performance and we have limit memory can use.

## RULE
- Path subscribe: sensors/+ <-- this topic will collect your data into collection on mongodb.
- topic at + should match with your key "type" in json
- Require this key in json : 
   - deviceID -> string : not specific template now
   - type -> string
   - data -> as object
   // and we can add location at this

## ENV
### 

```bash
# Set up your server connect to mqtt
TOS_IPADDRESS="<your_mqtt_ip_address>"
TOS_PORT="<your_mqtt_port>"

# Path as you need to store log
TOS_LOG_PATH="<your_log_path>"
# Base file name example -> TOS.log
TOS_LOG_BASENAME="<your_log_basename>"
# Default set it = 'log'
TOS_LOG_SUFFIX="<your_log_suffix>"

# Set up your db now I use it with mgdb and we'll add other db in the futures.
TOS_CON_MONGODB_PTC="<your_mongodb_protocol>"
TOS_CON_MONGODB_USERNAME="<your_mongodb_username>"
TOS_CON_MONGODB_PASSWORD="<your_mongodb_password>"
TOS_CON_MONGODB_DB_NAME="<your_mongodb_dbname>"
TOS_CON_MONGODB_IPADDRESS="<your_mongodb_ipaddress>"
TOS_CON_MONGODB_PORT="<your_mongodb_port>"
```
