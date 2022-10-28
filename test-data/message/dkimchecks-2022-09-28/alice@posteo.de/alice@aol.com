Authentication-Results: posteo.de; dmarc=pass (p=reject dis=none) header.from=aol.com
Authentication-Results: posteo.de; spf=pass smtp.mailfrom=aol.com
Authentication-Results: posteo.de;
	dkim=pass (2048-bit key) header.d=aol.com header.i=@aol.com header.b=GjnZ7bT0;
	dkim-atps=neutral
From: <alice@aol.com>
To: <alice@posteo.de>
