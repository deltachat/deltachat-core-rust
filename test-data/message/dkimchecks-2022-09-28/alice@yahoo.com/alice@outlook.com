Authentication-Results: atlas219.free.mail.bf1.yahoo.com;
 dkim=pass header.i=@outlook.com header.s=selector1;
 spf=pass smtp.mailfrom=outlook.com;
 dmarc=pass(p=NONE,sp=QUARANTINE) header.from=outlook.com;
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: <alice@yahoo.com>
