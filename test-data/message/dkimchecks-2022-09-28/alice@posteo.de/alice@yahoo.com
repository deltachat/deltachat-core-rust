Authentication-Results: posteo.de; dmarc=pass (p=reject dis=none) header.from=yahoo.com
Authentication-Results: posteo.de; spf=pass smtp.mailfrom=yahoo.com
Authentication-Results: posteo.de;
	dkim=pass (2048-bit key) header.d=yahoo.com header.i=@yahoo.com header.b=XTEPlzFO;
	dkim-atps=neutral
From: <alice@yahoo.com>
To: <alice@posteo.de>
