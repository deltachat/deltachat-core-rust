Authentication-Results: atlas206.aol.mail.ne1.yahoo.com;
 dkim=unknown;
 spf=none smtp.mailfrom=delta.blinzeln.de;
 dmarc=unknown header.from=delta.blinzeln.de;
From: authresadding-attacker@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
