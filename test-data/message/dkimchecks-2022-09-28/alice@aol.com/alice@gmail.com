Authentication-Results: atlas-baseline-production.v2-mail-prod1-gq1.omega.yahoo.com;
 dkim=pass header.i=@gmail.com header.s=20210112;
 spf=pass smtp.mailfrom=gmail.com;
 dmarc=pass(p=NONE,sp=QUARANTINE) header.from=gmail.com;
From: <alice@gmail.com>
To: <alice@aol.com>
