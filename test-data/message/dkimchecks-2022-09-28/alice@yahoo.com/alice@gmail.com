Authentication-Results: atlas216.free.mail.bf1.yahoo.com;
 dkim=pass header.i=@gmail.com header.s=20210112;
 spf=pass smtp.mailfrom=gmail.com;
 dmarc=pass(p=NONE,sp=QUARANTINE) header.from=gmail.com;
From: <alice@gmail.com>
To: <alice@yahoo.com>
