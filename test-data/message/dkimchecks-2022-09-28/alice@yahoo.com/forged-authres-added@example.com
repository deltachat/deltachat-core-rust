Authentication-Results: atlas324.free.mail.ne1.yahoo.com;
 dkim=unknown;
 spf=none smtp.mailfrom=delta.blinzeln.de;
 dmarc=unknown header.from=delta.blinzeln.de;
From: forged-authres-added@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
