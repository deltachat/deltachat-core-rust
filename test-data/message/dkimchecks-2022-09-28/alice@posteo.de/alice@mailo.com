Authentication-Results: posteo.de; dmarc=pass (p=none dis=none) header.from=mailo.com
Authentication-Results: posteo.de; spf=pass smtp.mailfrom=mailo.com
Authentication-Results: posteo.de;
	dkim=pass (1024-bit key) header.d=mailo.com header.i=@mailo.com header.b=Ye7KpuTx;
	dkim-atps=neutral
From: <alice@mailo.com>
To: <alice@posteo.de>
