Authentication-Results: posteo.de; dmarc=pass (p=none dis=none) header.from=outlook.com
Authentication-Results: posteo.de; spf=pass smtp.mailfrom=outlook.com
Authentication-Results: posteo.de;
	dkim=pass (2048-bit key) header.d=outlook.com header.i=@outlook.com header.b=uk70iBwu;
	dkim-atps=neutral
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: <alice@posteo.de>
