2. Mailing lists are now only detected by the ListId header, because how should DC sort mailing lists if the ListId header is unset and only the precedence header says it's a mailing list (?). We could just hide these emails (with "junk" or "list" precedence), as we did before, if there is a reason to do so. 

   I do not actually understand why such emails were hidden in the first place, though; if there is an automatic answer stating that someone is out of office or if an email could not be delivered (these are the usual reasons why such emails are sent), I would want to know about this as a user.
   
  Björn: to the ListId-header: it is already an improvement to use only that and to keep ignoring mails with Precedence-header. historically, i ignored all these mails to reduce noise that is created from them (eg. contacts should not be added to the suggestion list...) and to be able to concentrate on other things first.
   
   
3. for `List-Id: foo <bla>`, bla now is the id and foo is the name. For GitHub and GitLab notifications this is bad because all notifications will go into one chat. Maybe we should instead take the "in-reply-to" header to find out what messages belong together. For normal mailing lists, of course, this will create a second chat if someone does not use the "Reply" button to write to that mailing list.

  Björn: well, having all notifications in one chat is not that bad. i would just follow the guidelines for now, use the ID and not the Name.


4. Currently a mailing list is shown as an empty group (ChatType `Group`) ("0 members"). 

   Maybe we should change the ChatType to `Single` because this way, the UI would fit better. Disadvantage: We can't show the different senders of the messages. 
   
   Or we introduce a `ChatType` `MailingList`. Very big disadvantage: We would have to adapt all UI project and it would be nice if we could keep the changes within the core.

5. Contacts from mailings lists stay "unknown" now and are not shown in the contacts suggestion list..
