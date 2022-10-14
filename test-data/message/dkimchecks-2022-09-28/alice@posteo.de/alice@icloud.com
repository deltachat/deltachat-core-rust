Authentication-Results: posteo.de; dmarc=pass (p=quarantine dis=none) header.from=icloud.com
Authentication-Results: posteo.de; spf=pass smtp.mailfrom=icloud.com
Authentication-Results: posteo.de;
	dkim=pass (2048-bit key) header.d=icloud.com header.i=@icloud.com header.b=r/M8U3nt;
	dkim-atps=neutral
From: <alice@icloud.com>
To: <alice@posteo.de>
