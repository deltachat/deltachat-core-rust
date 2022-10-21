// AUTO-GENERATED by typescript-type-def

export type U32=number;
export type Usize=number;
export type Event=(({
/**
 * The library-user may write an informational string to the log.
 * 
 * This event should *not* be reported to the end-user using a popup or something like
 * that.
 */
"type":"Info";}&{"msg":string;})|({
/**
 * Emitted when SMTP connection is established and login was successful.
 */
"type":"SmtpConnected";}&{"msg":string;})|({
/**
 * Emitted when IMAP connection is established and login was successful.
 */
"type":"ImapConnected";}&{"msg":string;})|({
/**
 * Emitted when a message was successfully sent to the SMTP server.
 */
"type":"SmtpMessageSent";}&{"msg":string;})|({
/**
 * Emitted when an IMAP message has been marked as deleted
 */
"type":"ImapMessageDeleted";}&{"msg":string;})|({
/**
 * Emitted when an IMAP message has been moved
 */
"type":"ImapMessageMoved";}&{"msg":string;})|({
/**
 * Emitted when an new file in the $BLOBDIR was created
 */
"type":"NewBlobFile";}&{"file":string;})|({
/**
 * Emitted when an file in the $BLOBDIR was deleted
 */
"type":"DeletedBlobFile";}&{"file":string;})|({
/**
 * The library-user should write a warning string to the log.
 * 
 * This event should *not* be reported to the end-user using a popup or something like
 * that.
 */
"type":"Warning";}&{"msg":string;})|({
/**
 * The library-user should report an error to the end-user.
 * 
 * As most things are asynchronous, things may go wrong at any time and the user
 * should not be disturbed by a dialog or so.  Instead, use a bubble or so.
 * 
 * However, for ongoing processes (eg. configure())
 * or for functions that are expected to fail (eg. autocryptContinueKeyTransfer())
 * it might be better to delay showing these events until the function has really
 * failed (returned false). It should be sufficient to report only the *last* error
 * in a messasge box then.
 */
"type":"Error";}&{"msg":string;})|({
/**
 * An action cannot be performed because the user is not in the group.
 * Reported eg. after a call to
 * setChatName(), setChatProfileImage(),
 * addContactToChat(), removeContactFromChat(),
 * and messages sending functions.
 */
"type":"ErrorSelfNotInGroup";}&{"msg":string;})|({
/**
 * Messages or chats changed.  One or more messages or chats changed for various
 * reasons in the database:
 * - Messages sent, received or removed
 * - Chats created, deleted or archived
 * - A draft has been set
 * 
 * `chatId` is set if only a single chat is affected by the changes, otherwise 0.
 * `msgId` is set if only a single message is affected by the changes, otherwise 0.
 */
"type":"MsgsChanged";}&{"chatId":U32;"msgId":U32;})|({
/**
 * Reactions for the message changed.
 */
"type":"ReactionsChanged";}&{"chatId":U32;"msgId":U32;"contactId":U32;})|({
/**
 * There is a fresh message. Typically, the user will show an notification
 * when receiving this message.
 * 
 * There is no extra #DC_EVENT_MSGS_CHANGED event send together with this event.
 */
"type":"IncomingMsg";}&{"chatId":U32;"msgId":U32;})|({
/**
 * Messages were seen or noticed.
 * chat id is always set.
 */
"type":"MsgsNoticed";}&{"chatId":U32;})|({
/**
 * A single message is sent successfully. State changed from  DC_STATE_OUT_PENDING to
 * DC_STATE_OUT_DELIVERED, see `Message.state`.
 */
"type":"MsgDelivered";}&{"chatId":U32;"msgId":U32;})|({
/**
 * A single message could not be sent. State changed from DC_STATE_OUT_PENDING or DC_STATE_OUT_DELIVERED to
 * DC_STATE_OUT_FAILED, see `Message.state`.
 */
"type":"MsgFailed";}&{"chatId":U32;"msgId":U32;})|({
/**
 * A single message is read by the receiver. State changed from DC_STATE_OUT_DELIVERED to
 * DC_STATE_OUT_MDN_RCVD, see `Message.state`.
 */
"type":"MsgRead";}&{"chatId":U32;"msgId":U32;})|({
/**
 * Chat changed.  The name or the image of a chat group was changed or members were added or removed.
 * Or the verify state of a chat has changed.
 * See setChatName(), setChatProfileImage(), addContactToChat()
 * and removeContactFromChat().
 * 
 * This event does not include ephemeral timer modification, which
 * is a separate event.
 */
"type":"ChatModified";}&{"chatId":U32;})|({
/**
 * Chat ephemeral timer changed.
 */
"type":"ChatEphemeralTimerModified";}&{"chatId":U32;"timer":U32;})|({
/**
 * Contact(s) created, renamed, blocked or deleted.
 * 
 * @param data1 (int) If set, this is the contact_id of an added contact that should be selected.
 */
"type":"ContactsChanged";}&{"contactId":(U32|null);})|({
/**
 * Location of one or more contact has changed.
 * 
 * @param data1 (u32) contact_id of the contact for which the location has changed.
 *     If the locations of several contacts have been changed,
 *     this parameter is set to `None`.
 */
"type":"LocationChanged";}&{"contactId":(U32|null);})|({
/**
 * Inform about the configuration progress started by configure().
 */
"type":"ConfigureProgress";}&{
/**
 * Progress.
 * 
 * 0=error, 1-999=progress in permille, 1000=success and done
 */
"progress":Usize;
/**
 * Progress comment or error, something to display to the user.
 */
"comment":(string|null);})|({
/**
 * Inform about the import/export progress started by imex().
 * 
 * @param data1 (usize) 0=error, 1-999=progress in permille, 1000=success and done
 * @param data2 0
 */
"type":"ImexProgress";}&{"progress":Usize;})|({
/**
 * A file has been exported. A file has been written by imex().
 * This event may be sent multiple times by a single call to imex().
 * 
 * A typical purpose for a handler of this event may be to make the file public to some system
 * services.
 * 
 * @param data2 0
 */
"type":"ImexFileWritten";}&{"path":string;})|({
/**
 * Progress information of a secure-join handshake from the view of the inviter
 * (Alice, the person who shows the QR code).
 * 
 * These events are typically sent after a joiner has scanned the QR code
 * generated by getChatSecurejoinQrCodeSvg().
 * 
 * @param data1 (int) ID of the contact that wants to join.
 * @param data2 (int) Progress as:
 *     300=vg-/vc-request received, typically shown as "bob@addr joins".
 *     600=vg-/vc-request-with-auth received, vg-member-added/vc-contact-confirm sent, typically shown as "bob@addr verified".
 *     800=vg-member-added-received received, shown as "bob@addr securely joined GROUP", only sent for the verified-group-protocol.
 *     1000=Protocol finished for this contact.
 */
"type":"SecurejoinInviterProgress";}&{"contactId":U32;"progress":Usize;})|({
/**
 * Progress information of a secure-join handshake from the view of the joiner
 * (Bob, the person who scans the QR code).
 * The events are typically sent while secureJoin(), which
 * may take some time, is executed.
 * @param data1 (int) ID of the inviting contact.
 * @param data2 (int) Progress as:
 *     400=vg-/vc-request-with-auth sent, typically shown as "alice@addr verified, introducing myself."
 *     (Bob has verified alice and waits until Alice does the same for him)
 */
"type":"SecurejoinJoinerProgress";}&{"contactId":U32;"progress":Usize;})|{
/**
 * The connectivity to the server changed.
 * This means that you should refresh the connectivity view
 * and possibly the connectivtiy HTML; see getConnectivity() and
 * getConnectivityHtml() for details.
 */
"type":"ConnectivityChanged";}|{"type":"SelfavatarChanged";}|({"type":"WebxdcStatusUpdate";}&{"msgId":U32;"statusUpdateSerial":U32;})|({
/**
 * Inform that a message containing a webxdc instance has been deleted
 */
"type":"WebxdcInstanceDeleted";}&{"msgId":U32;}));
