Authentication-Results: atlas211.aol.mail.bf1.yahoo.com;
 dkim=pass header.i=@mailo.com header.s=mailo;
 spf=pass smtp.mailfrom=mailo.com;
 dmarc=pass(p=NONE) header.from=mailo.com;
From: <alice@mailo.com>
To: <alice@aol.com>
