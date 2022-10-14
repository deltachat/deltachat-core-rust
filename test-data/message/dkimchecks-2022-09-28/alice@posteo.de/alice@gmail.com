Authentication-Results: posteo.de; dmarc=pass (p=none dis=none) header.from=gmail.com
Authentication-Results: posteo.de; spf=pass smtp.mailfrom=gmail.com
Authentication-Results: posteo.de;
	dkim=pass (2048-bit key) header.d=gmail.com header.i=@gmail.com header.b=SJjarA70;
	dkim-atps=neutral
From: <alice@gmail.com>
To: <alice@posteo.de>
