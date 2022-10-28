Authentication-Results: atlas-production.v2-mail-prod1-gq1.omega.yahoo.com;
 dkim=pass header.i=@fastmail.com header.s=fm2;
 dkim=pass header.i=@messagingengine.com header.s=fm2;
 spf=pass smtp.mailfrom=fastmail.com;
 dmarc=pass(p=NONE,sp=NONE) header.from=fastmail.com;
From: <alice@fastmail.com>
To: <alice@yahoo.com>
