Authentication-Results: atlas112.aol.mail.bf1.yahoo.com;
 dkim=pass header.i=@hotmail.com header.s=selector1;
 spf=pass smtp.mailfrom=hotmail.com;
 dmarc=pass(p=NONE) header.from=hotmail.com;
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@hotmail.com>
To: <alice@aol.com>
