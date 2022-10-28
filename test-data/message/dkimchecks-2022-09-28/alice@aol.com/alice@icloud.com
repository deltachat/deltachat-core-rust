Authentication-Results: atlas101.aol.mail.bf1.yahoo.com;
 dkim=pass header.i=@icloud.com header.s=1a1hai;
 spf=pass smtp.mailfrom=icloud.com;
 dmarc=pass(p=QUARANTINE,sp=QUARANTINE) header.from=icloud.com;
From: <alice@icloud.com>
To: <alice@aol.com>
