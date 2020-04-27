1. Mailing lists with Precedence-header and without List-Id are shown as normal messages.
   
2. for `List-Id: foo <bla>`, bla now is the id and foo is the name. If foo is not present, bla is taken as the name as well. For GitHub and GitLab notifications all notifications will go into one chat. To distinguish, all subjects are shown in mailing lists.

3. Currently a mailing list is shown as a group with SELF and the List-Id. I could not find any stable way to get a mail address that could be called the "mailing list address". To get this to work, I disabled the may_be_valid_addr check. (to be discussed...)

4. Contacts from mailings lists stay "unknown" and are not shown in the contacts suggestion list.

5. In general, only the From: address is taken as the author. If the domain matches with the List-Id domain (like `deltachat.github.com` and `Hocuri <notifications@notification.github.com>`) then the display name is added to the email address -> `Hocuri - notifications@notification.github.com`. This is not really nice, but as the UIs distinguish bettween contacts only based on the address this was the only way I could find without changing the API. 

6. You can't send to mailing lists.

7. TODO: Block a mailing list (currently only the sender can be blocked and when someone else writes to the mailing list then it appears in Contact Requests again).
