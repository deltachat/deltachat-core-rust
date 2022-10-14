Authentication-Results: mx.infomaniak.com; dmarc=pass (p=none dis=none) header.from=mailo.com
Authentication-Results: mx.infomaniak.com;
	dkim=pass (1024-bit key; unprotected) header.d=mailo.com header.i=@mailo.com header.b="W4AVjC6K";
	dkim-atps=neutral
From: <alice@mailo.com>
To: <alice@ik.me>
Authentication-Results: mx.infomaniak.com; spf=pass smtp.mailfrom=mailo.com
