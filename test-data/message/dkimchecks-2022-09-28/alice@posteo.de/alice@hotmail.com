Authentication-Results: posteo.de; dmarc=pass (p=none dis=none) header.from=hotmail.com
Authentication-Results: posteo.de; spf=pass smtp.mailfrom=hotmail.com
Authentication-Results: posteo.de;
	dkim=pass (2048-bit key) header.d=hotmail.com header.i=@hotmail.com header.b=aqo8efk9;
	dkim-atps=neutral
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@hotmail.com>
To: <alice@posteo.de>
