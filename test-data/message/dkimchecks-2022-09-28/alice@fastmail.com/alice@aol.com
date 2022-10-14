ARC-Authentication-Results: i=1; mx1.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=sonic314-20.consmr.mail.ir2.yahoo.com
    policy.ptr=sonic314-20.consmr.mail.ir2.yahoo.com;
    bimi=none (No BIMI records found);
    arc=none (no signatures found);
    dkim=pass (2048-bit rsa key sha256) header.d=aol.com header.i=@aol.com
    header.b=Y+EgdIPN header.a=rsa-sha256 header.s=a2048 x-bits=2048;
    dmarc=pass policy.published-domain-policy=reject
    policy.applied-disposition=none policy.evaluated-disposition=none
    (p=reject,d=none,d.eval=none) policy.policy-from=p
    header.from=aol.com;
    iprev=pass smtp.remote-ip=77.238.177.146
    (sonic314-20.consmr.mail.ir2.yahoo.com);
    spf=pass smtp.mailfrom=alice@aol.com
    smtp.helo=sonic314-20.consmr.mail.ir2.yahoo.com
Authentication-Results: mx1.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=sonic314-20.consmr.mail.ir2.yahoo.com
      policy.ptr=sonic314-20.consmr.mail.ir2.yahoo.com
Authentication-Results: mx1.messagingengine.com;
    bimi=none (No BIMI records found)
Authentication-Results: mx1.messagingengine.com;
    arc=none (no signatures found)
Authentication-Results: mx1.messagingengine.com;
    dkim=pass (2048-bit rsa key sha256) header.d=aol.com header.i=@aol.com
      header.b=Y+EgdIPN header.a=rsa-sha256 header.s=a2048 x-bits=2048;
    dmarc=pass policy.published-domain-policy=reject
      policy.applied-disposition=none policy.evaluated-disposition=none
      (p=reject,d=none,d.eval=none) policy.policy-from=p
      header.from=aol.com;
    iprev=pass smtp.remote-ip=77.238.177.146
      (sonic314-20.consmr.mail.ir2.yahoo.com);
    spf=pass smtp.mailfrom=alice@aol.com
      smtp.helo=sonic314-20.consmr.mail.ir2.yahoo.com
From: <alice@aol.com>
To: <alice@fastmail.com>
