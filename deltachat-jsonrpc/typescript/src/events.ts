// Manual update might be required from time to time as this is NOT generated

export enum Event_TypeID {
  INFO = 100,
  SMTP_CONNECTED = 101,
  IMAP_CONNECTED = 102,
  SMTP_MESSAGE_SENT = 103,
  IMAP_MESSAGE_DELETED = 104,
  IMAP_MESSAGE_MOVED = 105,
  NEW_BLOB_FILE = 150,
  DELETED_BLOB_FILE = 151,
  WARNING = 300,
  ERROR = 400,
  ERROR_NETWORK = 401,
  ERROR_SELF_NOT_IN_GROUP = 410,
  MSGS_CHANGED = 2000,
  INCOMING_MSG = 2005,
  MSGS_NOTICED = 2008,
  MSG_DELIVERED = 2010,
  MSG_FAILED = 2012,
  MSG_READ = 2015,
  CHAT_MODIFIED = 2020,
  CHAT_EPHEMERAL_TIMER_MODIFIED = 2021,
  CONTACTS_CHANGED = 2030,
  LOCATION_CHANGED = 2035,
  CONFIGURE_PROGRESS = 2041,
  IMEX_PROGRESS = 2051,
  IMEX_FILE_WRITTEN = 2052,
  SECUREJOIN_INVITER_PROGRESS = 2060,
  SECUREJOIN_JOINER_PROGRESS = 2061,
  CONNECTIVITY_CHANGED = 2100,
}

export function eventIdToName(
  event_id: number
): keyof typeof Event_TypeID | "UNKNOWN_EVENT" {
  const name = Event_TypeID[event_id];
  if (name) {
    return name as keyof typeof Event_TypeID;
  } else {
    console.error("Unknown Event id:", event_id);
    return "UNKNOWN_EVENT";
  }
}
